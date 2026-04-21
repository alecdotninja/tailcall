use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    fold::{self, Fold},
    parse2, parse_quote, Error, Expr, ExprBlock, ExprCall, ExprIf, ExprMacro, ExprMatch, ExprPath,
    ExprReturn, ExprTry, Ident, Item, ItemFn, ItemMacro, Pat, PatIdent, PatType, Stmt,
};

use crate::call_syntax::is_tailcall_macro;

pub fn lower_self_tail_loop(item_fn: &ItemFn) -> Result<TokenStream, Error> {
    let arg_idents = function_arg_idents(item_fn)?;
    let mut lowerer = LoopLowerer {
        fn_ident: item_fn.sig.ident.clone(),
        arg_idents: arg_idents.clone(),
        temp_counter: 0,
        error: None,
    };

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

fn function_arg_idents(item_fn: &ItemFn) -> Result<Vec<Ident>, Error> {
    item_fn
        .sig
        .inputs
        .iter()
        .map(|fn_arg| match fn_arg {
            syn::FnArg::Receiver(receiver) => Err(Error::new_spanned(
                receiver,
                "loop lowering only supports free functions",
            )),
            syn::FnArg::Typed(PatType { pat, .. }) => match &**pat {
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
            },
        })
        .collect()
}

struct LoopLowerer {
    fn_ident: Ident,
    arg_idents: Vec<Ident>,
    temp_counter: usize,
    error: Option<Error>,
}

impl LoopLowerer {
    fn lower_tail_block(&mut self, mut block: syn::Block) -> syn::Block {
        let last_stmt = block.stmts.pop();
        block.stmts = block
            .stmts
            .into_iter()
            .map(|stmt| self.fold_stmt(stmt))
            .collect();

        if let Some(stmt) = last_stmt {
            block.stmts.push(match stmt {
                Stmt::Expr(expr) => Stmt::Expr(self.lower_tail_expr(expr)),
                Stmt::Semi(expr, semi) => Stmt::Semi(self.fold_expr(expr), semi),
                Stmt::Local(local) => Stmt::Local(self.fold_local(local)),
                Stmt::Item(Item::Macro(item_macro)) => Stmt::Expr(self.lower_tail_item_macro(item_macro)),
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
                else_branch: else_branch.map(|(else_token, expr)| {
                    (else_token, Box::new(self.lower_tail_expr(*expr)))
                }),
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
        let expr_call = match parse2::<ExprCall>(expr_macro.mac.tokens.clone()) {
            Ok(expr_call) => expr_call,
            Err(error) => {
                self.reject(error);
                return parse_quote! { continue };
            }
        };

        match &*expr_call.func {
            Expr::Path(ExprPath { path, .. }) if path.is_ident(&self.fn_ident) => {}
            _ => {
                self.reject(Error::new_spanned(
                    expr_call,
                    "loop lowering only supports direct self tail calls",
                ));
                return parse_quote! { continue };
            }
        }

        if expr_call.args.len() != self.arg_idents.len() {
            self.reject(Error::new_spanned(
                expr_call,
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

        let assignments: Vec<Stmt> = expr_call
            .args
            .into_iter()
            .zip(temp_idents.iter())
            .map(|(arg, temp_ident)| {
                let arg = self.fold_expr(arg);
                parse_quote! {
                    let #temp_ident = #arg;
                }
            })
            .collect();

        let rebinds: Vec<Stmt> = self
            .arg_idents
            .iter()
            .zip(temp_idents.iter())
            .map(|(ident, temp_ident)| {
                parse_quote! {
                    #ident = #temp_ident;
                }
            })
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

    fn lower_tail_item_macro(&mut self, item_macro: ItemMacro) -> Expr {
        if is_tailcall_macro(&item_macro.mac.path) {
            return self.lower_self_tailcall(ExprMacro {
                attrs: item_macro.attrs,
                mac: item_macro.mac,
            });
        }

        self.reject(Error::new_spanned(
            item_macro,
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

    use super::lower_self_tail_loop;

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
}
