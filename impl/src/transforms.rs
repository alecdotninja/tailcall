use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{fold::Fold, *};

use super::helpers::*;

pub enum TailRetType {
    Default,
    Result,
}

pub fn apply_fn_tailcall_transform(item_fn: ItemFn, ret_type: TailRetType) -> ItemFn {
    FnTailcallTransformer::new(ret_type).fold_item_fn(item_fn)
}

struct FnTailcallTransformer {
    ret_type: TailRetType
}

impl FnTailcallTransformer {
    pub fn new(ret_type: TailRetType) -> Self {
        Self {ret_type}
    }
}

fn wrap_ok(ret_type: &TailRetType) -> TokenStream {
    match ret_type {
        TailRetType::Default => quote!(),
        TailRetType::Result => quote!(Ok),
    }
}


impl Fold for FnTailcallTransformer {
    fn fold_item_fn(&mut self, item_fn: ItemFn) -> ItemFn {
        let ItemFn {
            attrs,
            vis,
            sig,
            block,
        } = item_fn;

        let input_pat_idents = sig.input_pat_idents();
        let input_idents = sig.input_idents();
        let block = apply_fn_tailcall_body_transform(&sig.ident, *block, &self.ret_type);

        let runner = match &self.ret_type {
            TailRetType::Default => quote!(tailcall::trampoline::run),
            TailRetType::Result => quote!(tailcall::trampoline::run_res),
        };
        let ok = wrap_ok(&self.ret_type);

        let block = parse_quote! {
            {
                #runner(
                    #[inline(always)] |(#(#input_pat_idents),*)| {
                        #ok(tailcall::trampoline::Finish(#block))
                    },
                    (#(#input_idents),*),
                )
            }
        };

        ItemFn {
            attrs,
            vis,
            sig,
            block,
        }
    }
}

pub fn apply_fn_tailcall_body_transform(fn_name_ident: &Ident, block: Block, ret_type: &TailRetType) -> Block {
    FnTailCallBodyTransformer::new(fn_name_ident, ret_type).fold_block(block)
}

struct FnTailCallBodyTransformer<'a> {
    fn_name_ident: &'a Ident,
    ret_type: &'a TailRetType,
}

impl<'a> FnTailCallBodyTransformer<'a> {
    pub fn new(fn_name_ident: &'a Ident, ret_type: &'a TailRetType) -> Self {
        Self {fn_name_ident, ret_type}
    }

    // `fn(X)` => `return Recurse(X)`
    fn try_rewrite_call_expr(&mut self, expr: &Expr) -> Option<Expr> {
        let ok = wrap_ok(&self.ret_type);
        if let Expr::Call(ExprCall { func, args, .. }) = expr {
            if let Expr::Path(ExprPath { ref path, .. }) = **func {
                if let Some(ident) = path.get_ident() {
                    if ident == self.fn_name_ident {
                        let args = self.fold_expr_tuple(parse_quote! { (#args) });

                        return Some(parse_quote! {
                            return #ok(tailcall::trampoline::Recurse(#args))
                        });
                    }
                }
            }
        }

        None
    }

    // `return fn(X)`   =>  `return Recurse(X)`
    // `return X`       =>  `return Finish(X)`
    fn try_rewrite_return_expr(&mut self, expr: &Expr) -> Option<Expr> {
        if let Expr::Return(ExprReturn { expr, .. }) = expr {
            let ok = wrap_ok(&self.ret_type);

            // TODO: Store in const
            let empty_tuple = parse_quote! { () };

            let expr = if let Some(expr) = expr {
                expr
            } else {
                &empty_tuple
            };

            return self.try_rewrite_expr(expr).or_else(|| {
                let expr = self.fold_expr(*expr.clone());

                Some(parse_quote! {
                    return #ok(tailcall::trampoline::Finish(#expr))
                })
            });
        }

        None
    }

    fn try_rewrite_expr(&mut self, expr: &Expr) -> Option<Expr> {
        self.try_rewrite_return_expr(expr)
            .or_else(|| self.try_rewrite_call_expr(expr))
    }
}

impl Fold for FnTailCallBodyTransformer<'_> {
    fn fold_expr(&mut self, expr: Expr) -> Expr {
        self.try_rewrite_expr(&expr)
            .unwrap_or_else(|| fold::fold_expr(self, expr))
    }

    fn fold_expr_closure(&mut self, expr_closure: ExprClosure) -> ExprClosure {
        // The meaning of the `return` keyword changes here -- stop transforming
        expr_closure
    }

    fn fold_item_fn(&mut self, item_fn: ItemFn) -> ItemFn {
        // The meaning of the `return` keyword changes here -- stop transforming
        item_fn
    }
}
