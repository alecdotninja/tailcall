use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    fold::{self, Fold},
    parse2, parse_quote, Error, Expr, ExprBlock, ExprCall, ExprIf, ExprMacro, ExprMatch,
    ExprMethodCall, ExprReturn, ExprTry, ImplItemMethod, ItemFn, Signature, Stmt,
};

use super::helpers::{
    function_argument_exprs, helper_method_call_tokens, helper_path_from_call, helper_signature,
    is_tailcall_macro, method_helper_signature,
};

pub fn apply_fn_tailcall_transform(item_fn: ItemFn) -> TokenStream {
    match TailcallTransform::new(item_fn).expand() {
        Ok(output) => output,
        Err(error) => error.to_compile_error(),
    }
}

pub fn expand_call_macro(tokens: TokenStream) -> TokenStream {
    if let Ok(expr_call) = parse2::<ExprCall>(tokens.clone()) {
        return match helper_path_from_call(&expr_call) {
            Ok(func) => {
                let args = expr_call.args;
                quote! { #func(#args) }
            }
            Err(error) => error.to_compile_error(),
        };
    }

    if let Ok(expr_method_call) = parse2::<ExprMethodCall>(tokens.clone()) {
        return match helper_method_call_tokens(&expr_method_call) {
            Ok(tokens) => tokens,
            Err(error) => error.to_compile_error(),
        };
    }

    Error::new(Span::call_site(), "tailcall::call! expects either `path(args...)` or `self.method(args...)`")
        .to_compile_error()
}

pub fn apply_method_tailcall_transform(method: ImplItemMethod) -> TokenStream {
    match TailcallMethodTransform::new(method).expand() {
        Ok(output) => output,
        Err(error) => error.to_compile_error(),
    }
}

struct TailcallTransform {
    item_fn: ItemFn,
}

struct TailcallMethodTransform {
    method: ImplItemMethod,
}

impl TailcallMethodTransform {
    fn new(method: ImplItemMethod) -> Self {
        Self { method }
    }

    fn expand(self) -> Result<TokenStream, Error> {
        let ImplItemMethod {
            attrs,
            vis,
            defaultness,
            sig,
            block,
        } = self.method;

        reject_unsupported_signature(&sig)?;

        let helper_sig = method_helper_signature(&sig)?;
        let helper_fn_ident = &helper_sig.ident;
        let helper_args = function_argument_exprs(&sig)?;
        let helper_block = TailPositionRewriter::rewrite(block)?;

        Ok(quote! {
            #(#attrs)*
            #defaultness #vis #sig {
                tailcall::trampoline::run(Self::#helper_fn_ident(#(#helper_args),*))
            }

            #[doc(hidden)]
            #[inline(always)]
            #helper_sig {
                tailcall::trampoline::call(move || #helper_block)
            }
        })
    }
}

impl TailcallTransform {
    fn new(item_fn: ItemFn) -> Self {
        Self { item_fn }
    }

    fn expand(self) -> Result<TokenStream, Error> {
        let ItemFn {
            attrs,
            vis,
            sig,
            block,
        } = self.item_fn;

        reject_unsupported_signature(&sig)?;

        let helper_sig = helper_signature(&sig);
        let helper_fn_ident = &helper_sig.ident;
        let helper_args = function_argument_exprs(&sig)?;
        let helper_block = TailPositionRewriter::rewrite(*block)?;

        Ok(quote! {
            #(#attrs)*
            #vis #sig {
                tailcall::trampoline::run(#helper_fn_ident(#(#helper_args),*))
            }

            #[doc(hidden)]
            #[inline(always)]
            #helper_sig {
                tailcall::trampoline::call(move || #helper_block)
            }
        })
    }
}

fn reject_unsupported_signature(sig: &Signature) -> Result<(), Error> {
    if sig.constness.is_some() {
        return Err(Error::new_spanned(
            &sig.constness,
            "#[tailcall] does not support const functions",
        ));
    }

    if sig.asyncness.is_some() {
        return Err(Error::new_spanned(
            &sig.asyncness,
            "#[tailcall] does not support async functions",
        ));
    }

    Ok(())
}

struct TailPositionRewriter {
    error: Option<Error>,
}

impl TailPositionRewriter {
    fn rewrite(block: syn::Block) -> Result<syn::Block, Error> {
        let mut rewriter = Self { error: None };
        let block = rewriter.rewrite_tail_block(block);

        match rewriter.error {
            Some(error) => Err(error),
            None => Ok(block),
        }
    }

    fn rewrite_tail_block(&mut self, mut block: syn::Block) -> syn::Block {
        let last_stmt = block.stmts.pop();
        block.stmts = block
            .stmts
            .into_iter()
            .map(|stmt| self.fold_stmt(stmt))
            .collect();

        if let Some(stmt) = last_stmt {
            block.stmts.push(match stmt {
                Stmt::Expr(expr) => Stmt::Expr(self.rewrite_tail_expr(expr)),
                Stmt::Semi(expr, semi) => Stmt::Semi(self.fold_expr(expr), semi),
                Stmt::Local(local) => Stmt::Local(self.fold_local(local)),
                Stmt::Item(item) => Stmt::Item(item),
            });
        }

        block
    }

    fn rewrite_tail_expr(&mut self, expr: Expr) -> Expr {
        match expr {
            Expr::Block(ExprBlock {
                attrs,
                label,
                block,
            }) => Expr::Block(ExprBlock {
                attrs,
                label,
                block: self.rewrite_tail_block(block),
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
                then_branch: self.rewrite_tail_block(then_branch),
                else_branch: else_branch
                    .map(|(else_token, expr)| (else_token, Box::new(self.rewrite_tail_expr(*expr)))),
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
                        arm.body = Box::new(self.rewrite_tail_expr(*arm.body));
                        arm
                    })
                    .collect(),
            }),
            Expr::Macro(expr_macro) if is_tailcall_macro(&expr_macro.mac.path) => {
                expand_call_expr(expr_macro)
            }
            expr => {
                let expr = self.fold_expr(expr);
                parse_quote! { tailcall::trampoline::done(#expr) }
            }
        }
    }

    fn reject(&mut self, error: Error) {
        if let Some(existing) = &mut self.error {
            existing.combine(error);
        } else {
            self.error = Some(error);
        }
    }
}

impl Fold for TailPositionRewriter {
    fn fold_expr(&mut self, expr: Expr) -> Expr {
        match expr {
            Expr::Return(ExprReturn {
                attrs,
                return_token,
                expr: Some(expr),
            }) => Expr::Return(ExprReturn {
                attrs,
                return_token,
                expr: Some(Box::new(self.rewrite_tail_expr(*expr))),
            }),
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
                expand_call_expr(expr_macro)
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

fn expand_call_expr(expr_macro: ExprMacro) -> Expr {
    parse2(expand_call_macro(expr_macro.mac.tokens)).expect("tailcall::call! should expand to an expression")
}
