use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{fold::Fold, parse2, parse_quote, *};

use super::helpers::{helper_ident, helper_path_for, is_tailcall_macro, FunctionArgs};

pub fn apply_fn_tailcall_transform(item_fn: ItemFn) -> TokenStream {
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = item_fn;

    let helper_sig = build_helper_signature(&sig);
    let helper_fn_ident = helper_ident(&sig.ident);
    let helper_args = sig.argument_exprs();
    let helper_block = apply_fn_tailcall_body_transform(*block);

    quote! {
        #(#attrs)*
        #vis #sig {
            tailcall::trampoline::run(#helper_fn_ident(#(#helper_args),*))
        }

        #[doc(hidden)]
        #[inline(always)]
        #helper_sig {
            tailcall::trampoline::call(move || #helper_block)
        }
    }
}

fn build_helper_signature(sig: &Signature) -> Signature {
    let mut helper_sig = sig.clone();
    let output_ty = output_type(&sig.output);
    let tailcall_lifetime = Lifetime::new("'tailcall", Span::call_site());

    helper_sig.ident = helper_ident(&sig.ident);
    helper_sig
        .generics
        .params
        .push(parse_quote!(#tailcall_lifetime));
    helper_sig.output = parse_quote! { -> tailcall::trampoline::Action<#tailcall_lifetime, #output_ty> };

    helper_sig = ElidedLifetimeRewriter {
        lifetime: tailcall_lifetime.clone(),
    }
    .fold_signature(helper_sig);

    let where_clause = helper_sig.generics.make_where_clause();
    for generic_param in &sig.generics.params {
        match generic_param {
            GenericParam::Type(type_param) => {
                let ident = &type_param.ident;
                where_clause
                    .predicates
                    .push(parse_quote!(#ident: #tailcall_lifetime));
            }
            GenericParam::Lifetime(lifetime_def) => {
                let lifetime = &lifetime_def.lifetime;
                where_clause
                    .predicates
                    .push(parse_quote!(#lifetime: #tailcall_lifetime));
            }
            GenericParam::Const(_) => {}
        }
    }

    helper_sig
}

fn output_type(output: &ReturnType) -> Type {
    match output {
        ReturnType::Default => parse_quote! { () },
        ReturnType::Type(_, ty) => (**ty).clone(),
    }
}

pub fn apply_fn_tailcall_body_transform(block: Block) -> Block {
    FnTailCallBodyTransformer::new().transform_block_tail(block)
}

pub fn expand_call_macro(tokens: TokenStream) -> TokenStream {
    let expr_call: ExprCall = parse2(tokens).expect("tailcall::call! expects a function call");
    let func = match *expr_call.func {
        Expr::Path(ExprPath { path, .. }) => helper_path_for(&path),
        _ => panic!("tailcall::call! expects a function path"),
    };
    let args = expr_call.args;

    quote! { #func(#args) }
}

struct FnTailCallBodyTransformer;

impl FnTailCallBodyTransformer {
    pub fn new() -> Self {
        Self
    }

    fn transform_tail_expr(&mut self, expr: Expr) -> Expr {
        match expr {
            Expr::Block(mut expr_block) => {
                expr_block.block = self.transform_block_tail(expr_block.block);
                Expr::Block(expr_block)
            }
            Expr::If(mut expr_if) => {
                expr_if.cond = Box::new(self.fold_expr(*expr_if.cond));
                expr_if.then_branch = self.transform_block_tail(expr_if.then_branch);
                expr_if.else_branch = expr_if.else_branch.map(|(else_token, else_expr)| {
                    (else_token, Box::new(self.transform_tail_expr(*else_expr)))
                });
                Expr::If(expr_if)
            }
            Expr::Match(mut expr_match) => {
                expr_match.expr = Box::new(self.fold_expr(*expr_match.expr));
                expr_match.arms = expr_match
                    .arms
                    .into_iter()
                    .map(|mut arm| {
                        if let Some((if_token, guard_expr)) = arm.guard.take() {
                            arm.guard = Some((if_token, Box::new(self.fold_expr(*guard_expr))));
                        }
                        arm.body = Box::new(self.transform_tail_expr(*arm.body));
                        arm
                    })
                    .collect();
                Expr::Match(expr_match)
            }
            Expr::Macro(expr_macro) if is_tailcall_macro(&expr_macro.mac.path) => {
                parse2(expand_call_macro(expr_macro.mac.tokens))
                    .expect("tailcall::call! should expand to an expression")
            }
            expr => {
                let expr = self.fold_expr(expr);
                parse_quote! { tailcall::trampoline::done(#expr) }
            }
        }
    }

    fn transform_block_tail(&mut self, mut block: Block) -> Block {
        let last_stmt = block.stmts.pop();
        block.stmts = block
            .stmts
            .into_iter()
            .map(|stmt| self.fold_stmt(stmt))
            .collect();

        if let Some(stmt) = last_stmt {
            block.stmts.push(match stmt {
                Stmt::Expr(expr) => Stmt::Expr(self.transform_tail_expr(expr)),
                Stmt::Semi(expr, semi) => Stmt::Semi(self.fold_expr(expr), semi),
                Stmt::Local(local) => Stmt::Local(self.fold_local(local)),
                Stmt::Item(item) => Stmt::Item(item),
            });
        }

        block
    }
}

impl Fold for FnTailCallBodyTransformer {
    fn fold_expr(&mut self, expr: Expr) -> Expr {
        match expr {
            Expr::Return(ExprReturn {
                attrs,
                return_token,
                expr: Some(expr),
            }) => Expr::Return(ExprReturn {
                attrs,
                return_token,
                expr: Some(Box::new(self.transform_tail_expr(*expr))),
            }),
            Expr::Try(ExprTry { attrs, expr, .. }) => {
                let expr = self.fold_expr(*expr);
                parse_quote! {
                    {
                        #(#attrs)*
                        match ::core::ops::Try::branch(#expr) {
                            ::core::ops::ControlFlow::Continue(value) => value,
                            ::core::ops::ControlFlow::Break(residual) => {
                                return tailcall::trampoline::done(
                                    ::core::ops::FromResidual::from_residual(residual)
                                );
                            }
                        }
                    }
                }
            }
            Expr::Macro(expr_macro) if is_tailcall_macro(&expr_macro.mac.path) => {
                parse2(expand_call_macro(expr_macro.mac.tokens))
                    .expect("tailcall::call! should expand to an expression")
            }
            expr => fold::fold_expr(self, expr),
        }
    }

    fn fold_expr_closure(&mut self, expr_closure: ExprClosure) -> ExprClosure {
        expr_closure
    }

    fn fold_item_fn(&mut self, item_fn: ItemFn) -> ItemFn {
        item_fn
    }
}

struct ElidedLifetimeRewriter {
    lifetime: Lifetime,
}

impl Fold for ElidedLifetimeRewriter {
    fn fold_type_reference(&mut self, mut ty_ref: TypeReference) -> TypeReference {
        ty_ref.elem = Box::new(self.fold_type(*ty_ref.elem));

        if ty_ref.lifetime.is_none() {
            ty_ref.lifetime = Some(self.lifetime.clone());
        }

        ty_ref
    }

    fn fold_item_fn(&mut self, item_fn: ItemFn) -> ItemFn {
        item_fn
    }
}
