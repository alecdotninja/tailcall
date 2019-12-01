use proc_macro2::{Ident, Span};
use syn::{fold::Fold, *};

use super::helpers::*;

pub fn apply_fn_tailcall_transform(item_fn: ItemFn) -> ItemFn {
    FnTailcallTransformer::new().fold_item_fn(item_fn)
}

struct FnTailcallTransformer;

impl FnTailcallTransformer {
    pub fn new() -> Self {
        Self {}
    }

    // TODO: It would be nice to export this from the crate instead of re-generating the
    //       same code inside of each transformed function.
    fn build_support_mod(&self) -> (Ident, ItemMod) {
        // FIXME: Sadly, `Span::def_site()` is not yet stable, so choose an ident which
        //        is unlikely to collide with user code.
        let namespace_ident = Ident::new("___tailcall___", Span::call_site());

        let support_mod = parse_quote! {
            mod #namespace_ident {
                pub enum Next<Input, Output> {
                    Recurse(Input),
                    Finish(Output),
                }

                pub use Next::*;

                #[inline(always)]
                pub fn run<Step, Input, Output>(step: Step, mut input: Input) -> Output
                    where Step: Fn(Input) -> Next<Input, Output>
                {
                    loop {
                        match step(input) {
                            Recurse(new_input) => {
                                input = new_input;
                                continue;
                            },
                            Finish(output) => {
                                break output;
                            }
                        }
                    }
                }
            }
        };

        (namespace_ident, support_mod)
    }

    fn build_run_expr(&self, namespace_ident: &Ident, sig: &Signature, block: Block) -> Expr {
        let block = apply_fn_tailcall_body_transform(namespace_ident, &sig.ident, block);

        let input_pat_idents = sig.input_pat_idents();
        let input_idents = sig.input_idents();

        parse_quote! {
            #namespace_ident::run(
                #[inline(always)] |(#(#input_pat_idents),*)| {
                    #namespace_ident::Finish(#block)
                },
                (#(#input_idents),*),
            )
        }
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

        let (namespace_ident, support_mod) = self.build_support_mod();
        let run_expr = self.build_run_expr(&namespace_ident, &sig, *block);

        let block = parse_quote! {
            {
                #support_mod
                #run_expr
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

pub fn apply_fn_tailcall_body_transform(
    namespace_ident: &Ident,
    fn_name_ident: &Ident,
    block: Block,
) -> Block {
    FnTailCallBodyTransformer::new(namespace_ident, fn_name_ident).fold_block(block)
}

struct FnTailCallBodyTransformer<'a> {
    namespace_ident: &'a Ident,
    fn_name_ident: &'a Ident,
}

impl<'a> FnTailCallBodyTransformer<'a> {
    pub fn new(namespace_ident: &'a Ident, fn_name_ident: &'a Ident) -> Self {
        Self {
            namespace_ident,
            fn_name_ident,
        }
    }

    // `fn(X)` => `return Recurse(X)`
    fn try_rewrite_call_expr(&mut self, expr: &Expr) -> Option<Expr> {
        if let Expr::Call(ExprCall { func, args, .. }) = expr {
            if let Expr::Path(ExprPath { ref path, .. }) = **func {
                if let Some(ident) = path.get_ident() {
                    if ident == self.fn_name_ident {
                        let namespace_ident = self.namespace_ident;
                        let args = self.fold_expr_tuple(parse_quote! { (#args) });

                        return Some(parse_quote! {
                            return #namespace_ident::Recurse(#args)
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
            // TODO: Store in const
            let empty_tuple = parse_quote! { () };

            let expr = if let Some(expr) = expr {
                expr
            } else {
                &empty_tuple
            };

            return self.try_rewrite_expr(expr).or_else(|| {
                let namespace_ident = self.namespace_ident;
                let expr = self.fold_expr(*expr.clone());

                Some(parse_quote! {
                    return #namespace_ident::Finish(#expr)
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
