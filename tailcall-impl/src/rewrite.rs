use syn::{
    fold::{self, Fold},
    parse2, parse_quote, Error, Expr, ExprBlock, ExprIf, ExprMacro, ExprMatch, ExprReturn, ExprTry,
    ItemFn, Stmt,
};

use crate::call_syntax::{expand_call_macro, is_tailcall_macro};

pub struct TailPositionRewriter {
    error: Option<Error>,
}

impl TailPositionRewriter {
    pub fn rewrite(block: syn::Block) -> Result<syn::Block, Error> {
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
                else_branch: else_branch.map(|(else_token, expr)| {
                    (else_token, Box::new(self.rewrite_tail_expr(*expr)))
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
                parse_quote! { tailcall::Thunk::value(#expr) }
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
    parse2(expand_call_macro(expr_macro.mac.tokens))
        .expect("tailcall::call! should expand to an expression")
}
