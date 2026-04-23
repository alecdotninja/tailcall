use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    fold::{self, Fold},
    parse2, parse_quote, Error, Expr, ExprBlock, ExprCall, ExprIf, ExprMacro, ExprMatch,
    ExprMethodCall, ExprPath, ExprReturn, ExprTry, FnArg, Ident, ImplItemFn, Item, ItemFn,
    Pat, PatIdent, PatType, Stmt, StmtMacro,
};

use crate::call_syntax::is_tailcall_macro;

pub fn lower_self_tail_loop(item_fn: &ItemFn) -> Result<TokenStream, Error> {
    let arg_idents = function_arg_idents(&item_fn.sig.inputs)?;
    let mut lowerer = LoopLowerer::for_function(item_fn.sig.ident.clone(), arg_idents.clone());
    let loop_block = lowerer.lower_tail_block(*item_fn.block.clone());
    let rebinding_stmts: Vec<Stmt> = arg_idents
        .iter()
        .map(|ident| parse_quote! { let mut #ident = #ident; })
        .collect();

    match lowerer.error {
        Some(error) => Err(error),
        None => Ok(quote! {
            #(#rebinding_stmts)*
            loop #loop_block
        }),
    }
}

pub fn lower_self_tail_method_loop(method: &ImplItemFn) -> Result<TokenStream, Error> {
    let arg_idents = function_arg_idents(&method.sig.inputs)?;
    let receiver_alias = Ident::new("__tailcall_self", Span::call_site());
    let mut lowerer = LoopLowerer::for_method(
        method.sig.ident.clone(),
        arg_idents.clone(),
        receiver_alias.clone(),
    );
    let loop_block = lowerer.lower_tail_block(method.block.clone());
    let rebinding_stmts: Vec<Stmt> = arg_idents
        .iter()
        .map(|ident| parse_quote! { let mut #ident = #ident; })
        .collect();

    match lowerer.error {
        Some(error) => Err(error),
        None => Ok(quote! {
            let #receiver_alias = self;
            #(#rebinding_stmts)*
            loop #loop_block
        }),
    }
}

fn function_arg_idents(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
) -> Result<Vec<Ident>, Error> {
    inputs
        .iter()
        .filter_map(|fn_arg| match fn_arg {
            FnArg::Receiver(_) => None,
            FnArg::Typed(PatType { pat, .. }) => Some(match &**pat {
                Pat::Ident(PatIdent {
                    attrs,
                    by_ref: None,
                    ident,
                    subpat: None,
                    ..
                }) if attrs.is_empty() => Ok(ident.clone()),
                pat => Err(Error::new_spanned(
                    pat,
                    "#[tailcall] only supports simple identifier arguments",
                )),
            }),
        })
        .collect()
}

enum LoopTarget {
    Function(Ident),
    Method {
        method_ident: Ident,
        receiver_alias: Ident,
    },
}

struct LoopLowerer {
    target: LoopTarget,
    arg_idents: Vec<Ident>,
    temp_counter: usize,
    error: Option<Error>,
}

impl LoopLowerer {
    fn for_function(fn_ident: Ident, arg_idents: Vec<Ident>) -> Self {
        Self {
            target: LoopTarget::Function(fn_ident),
            arg_idents,
            temp_counter: 0,
            error: None,
        }
    }

    fn for_method(method_ident: Ident, arg_idents: Vec<Ident>, receiver_alias: Ident) -> Self {
        Self {
            target: LoopTarget::Method {
                method_ident,
                receiver_alias,
            },
            arg_idents,
            temp_counter: 0,
            error: None,
        }
    }

    fn lower_tail_block(&mut self, mut block: syn::Block) -> syn::Block {
        let last_stmt = block.stmts.pop();
        block.stmts = block
            .stmts
            .into_iter()
            .map(|stmt| self.fold_stmt(stmt))
            .collect();

        if let Some(stmt) = last_stmt {
            block.stmts.push(match stmt {
                Stmt::Expr(expr, None) => Stmt::Expr(self.lower_tail_expr(expr), None),
                Stmt::Expr(expr, semi) => Stmt::Expr(self.fold_expr(expr), semi),
                Stmt::Local(local) => Stmt::Local(self.fold_local(local)),
                Stmt::Item(Item::Macro(item_macro)) => Stmt::Item(Item::Macro(item_macro)),
                Stmt::Macro(stmt_macro) => Stmt::Expr(self.lower_tail_stmt_macro(stmt_macro), None),
                Stmt::Item(item) => Stmt::Item(item),
            });
        }

        block
    }

    fn lower_tail_expr(&mut self, expr: Expr) -> Expr {
        match expr {
            Expr::Return(ExprReturn {
                attrs: _,
                return_token: _,
                expr: Some(expr),
            }) => self.lower_tail_expr(*expr),
            Expr::Block(ExprBlock {
                attrs,
                label,
                block,
            }) => Expr::Block(ExprBlock {
                attrs,
                label,
                block: self.lower_tail_block(block),
            }),
            Expr::If(ExprIf {
                attrs,
                if_token,
                cond,
                then_branch,
                else_branch,
            }) => Expr::If(ExprIf {
                attrs,
                if_token,
                cond: Box::new(self.fold_expr(*cond)),
                then_branch: self.lower_tail_block(then_branch),
                else_branch: else_branch
                    .map(|(else_token, expr)| (else_token, Box::new(self.lower_tail_expr(*expr)))),
            }),
            Expr::Match(ExprMatch {
                attrs,
                match_token,
                expr,
                brace_token,
                arms,
            }) => Expr::Match(ExprMatch {
                attrs,
                match_token,
                expr: Box::new(self.fold_expr(*expr)),
                brace_token,
                arms: arms
                    .into_iter()
                    .map(|mut arm| {
                        if let Some((if_token, guard)) = arm.guard.take() {
                            arm.guard = Some((if_token, Box::new(self.fold_expr(*guard))));
                        }
                        arm.body = Box::new(self.lower_tail_expr(*arm.body));
                        arm
                    })
                    .collect(),
            }),
            Expr::Macro(expr_macro) if is_tailcall_macro(&expr_macro.mac.path) => {
                self.lower_self_tailcall(expr_macro)
            }
            expr => {
                let expr = self.fold_expr(expr);
                parse_quote! { return #expr }
            }
        }
    }

    fn lower_self_tailcall(&mut self, expr_macro: ExprMacro) -> Expr {
        match &self.target {
            LoopTarget::Function(fn_ident) => {
                let expr_call = match parse2::<ExprCall>(expr_macro.mac.tokens.clone()) {
                    Ok(expr_call) => expr_call,
                    Err(error) => {
                        self.reject(error);
                        return parse_quote! { continue };
                    }
                };

                match &*expr_call.func {
                    Expr::Path(ExprPath { path, .. }) if path.is_ident(fn_ident) => {}
                    _ => {
                        self.reject(Error::new_spanned(
                            expr_call,
                            "loop lowering only supports direct self tail calls",
                        ));
                        return parse_quote! { continue };
                    }
                }

                self.lower_tailcall_args(expr_call.args.into_iter().collect())
            }
            LoopTarget::Method { method_ident, .. } => {
                let expr_method_call = match parse2::<ExprMethodCall>(expr_macro.mac.tokens.clone())
                {
                    Ok(expr_method_call) => expr_method_call,
                    Err(error) => {
                        self.reject(error);
                        return parse_quote! { continue };
                    }
                };

                if !matches!(
                    &*expr_method_call.receiver,
                    Expr::Path(ExprPath { path, .. }) if path.is_ident("self")
                ) || expr_method_call.method != *method_ident
                {
                    self.reject(Error::new_spanned(
                        expr_method_call,
                        "loop lowering only supports direct self tail calls on `self`",
                    ));
                    return parse_quote! { continue };
                }

                self.lower_tailcall_args(expr_method_call.args.into_iter().collect())
            }
        }
    }

    fn lower_tailcall_args(&mut self, args: Vec<Expr>) -> Expr {
        if args.len() != self.arg_idents.len() {
            self.reject(Error::new(
                Span::call_site(),
                "tailcall::call! argument count must match the function signature",
            ));
            return parse_quote! { continue };
        }

        let temp_idents: Vec<Ident> = (0..self.arg_idents.len())
            .map(|_| {
                let ident = Ident::new(
                    &format!("__tailcall_next_{}", self.temp_counter),
                    Span::call_site(),
                );
                self.temp_counter += 1;
                ident
            })
            .collect();

        let assignments: Vec<Stmt> = args
            .into_iter()
            .zip(temp_idents.iter())
            .map(|(arg, temp_ident)| {
                let arg = self.fold_expr(arg);
                parse_quote! { let #temp_ident = #arg; }
            })
            .collect();

        let rebinds: Vec<Stmt> = self
            .arg_idents
            .iter()
            .zip(temp_idents.iter())
            .map(|(ident, temp_ident)| parse_quote! { #ident = #temp_ident; })
            .collect();

        let mut stmts = assignments;
        stmts.extend(rebinds);
        stmts.push(parse_quote! { continue; });

        Expr::Block(ExprBlock {
            attrs: Vec::new(),
            label: None,
            block: syn::Block {
                brace_token: Default::default(),
                stmts,
            },
        })
    }

    fn lower_tail_stmt_macro(&mut self, stmt_macro: StmtMacro) -> Expr {
        if is_tailcall_macro(&stmt_macro.mac.path) {
            return self.lower_self_tailcall(ExprMacro {
                attrs: stmt_macro.attrs,
                mac: stmt_macro.mac,
            });
        }

        self.reject(Error::new_spanned(
            stmt_macro,
            "tail-position macro items are not supported in loop lowering",
        ));
        parse_quote! { continue }
    }

    fn reject(&mut self, error: Error) {
        if let Some(existing) = &mut self.error {
            existing.combine(error);
        } else {
            self.error = Some(error);
        }
    }
}

impl Fold for LoopLowerer {
    fn fold_expr(&mut self, expr: Expr) -> Expr {
        match expr {
            Expr::Try(ExprTry {
                attrs,
                expr,
                question_token,
            }) => {
                self.reject(Error::new_spanned(
                    question_token,
                    "the `?` operator is not supported inside #[tailcall] functions on stable Rust; use `match` or explicit early returns instead",
                ));

                Expr::Try(ExprTry {
                    attrs,
                    expr: Box::new(self.fold_expr(*expr)),
                    question_token,
                })
            }
            Expr::Macro(expr_macro) if is_tailcall_macro(&expr_macro.mac.path) => {
                self.reject(Error::new_spanned(
                    &expr_macro,
                    "tailcall::call! must be used in tail position",
                ));
                self.lower_self_tailcall(expr_macro)
            }
            expr => fold::fold_expr(self, expr),
        }
    }

    fn fold_expr_method_call(&mut self, expr_method_call: ExprMethodCall) -> ExprMethodCall {
        if matches!(
            (&self.target, &*expr_method_call.receiver),
            (
                LoopTarget::Method { method_ident, .. },
                Expr::Path(ExprPath { path, .. })
            ) if path.is_ident("self") && expr_method_call.method == *method_ident
        ) {
            self.reject(Error::new_spanned(
                &expr_method_call,
                "tailcall::call! must be used in tail position",
            ));
        }

        fold::fold_expr_method_call(self, expr_method_call)
    }

    fn fold_expr_path(&mut self, mut expr_path: ExprPath) -> ExprPath {
        if let LoopTarget::Method { receiver_alias, .. } = &self.target {
            if expr_path.path.is_ident("self") {
                expr_path.path = parse_quote! { #receiver_alias };
            }
        }

        expr_path
    }

    fn fold_expr_closure(&mut self, expr: syn::ExprClosure) -> syn::ExprClosure {
        expr
    }

    fn fold_item_fn(&mut self, item_fn: ItemFn) -> ItemFn {
        item_fn
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::{lower_self_tail_loop, lower_self_tail_method_loop};

    #[test]
    fn lowers_simple_self_tail_recursion() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn countdown(n: u64) -> u64 {
                if n > 0 {
                    tailcall::call! { countdown(n - 1) }
                } else {
                    0
                }
            }
        };

        lower_self_tail_loop(&item_fn).expect("loop lowering should succeed");
    }

    #[test]
    fn lowers_self_tail_recursion_with_computed_arguments() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn scramble_tailcall_go(n: u64, state: u64) -> u64 {
                if n > 0 {
                    tailcall::call! { scramble_tailcall_go(n - 1, scramble_step(state, n)) }
                } else {
                    state
                }
            }
        };

        lower_self_tail_loop(&item_fn).expect("loop lowering should succeed");
    }

    #[test]
    fn lowers_simple_self_tail_recursive_method() {
        let method: syn::ImplItemFn = parse_quote! {
            fn countdown(&self, n: u32) -> u32 {
                if n > 0 {
                    tailcall::call! { self.countdown(n - 1) }
                } else {
                    0
                }
            }
        };

        lower_self_tail_method_loop(&method).expect("method loop lowering should succeed");
    }
}
