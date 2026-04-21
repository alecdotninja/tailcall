use syn::{
    parse2,
    visit::{self, Visit},
    Expr, ExprCall, ExprMacro, ExprPath, FnArg, Ident, ItemFn, ItemMacro, Pat, PatIdent, PatType,
    Path, ReturnType, Type,
};

use crate::call_syntax::is_tailcall_macro;

pub fn is_simple_self_tail_recursive(item_fn: &ItemFn) -> bool {
    let (eligible, saw_self_tailcall) = analyze(item_fn);
    eligible && saw_self_tailcall
}

fn analyze(item_fn: &ItemFn) -> (bool, bool) {
    if !item_fn.sig.generics.params.is_empty() || item_fn.sig.generics.where_clause.is_some() {
        return (false, false);
    }

    if returns_result(&item_fn.sig.output) {
        return (false, false);
    }

    let mut analyzer = SelfTailAnalyzer {
        fn_ident: &item_fn.sig.ident,
        arg_idents: function_arg_idents(item_fn),
        eligible: true,
        saw_self_tailcall: false,
    };
    analyzer.visit_block(&item_fn.block);
    (analyzer.eligible, analyzer.saw_self_tailcall)
}

fn returns_result(output: &ReturnType) -> bool {
    match output {
        ReturnType::Default => false,
        ReturnType::Type(_, ty) => match &**ty {
            Type::Path(type_path) => type_path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "Result"),
            _ => false,
        },
    }
}

struct SelfTailAnalyzer<'a> {
    fn_ident: &'a Ident,
    arg_idents: Vec<Ident>,
    eligible: bool,
    saw_self_tailcall: bool,
}

impl SelfTailAnalyzer<'_> {
    fn is_self_path(&self, path: &Path) -> bool {
        path.is_ident(self.fn_ident)
    }

    fn is_argument_ident(&self, ident: &Ident) -> bool {
        self.arg_idents.iter().any(|arg_ident| arg_ident == ident)
    }
}

fn function_arg_idents(item_fn: &ItemFn) -> Vec<Ident> {
    item_fn
        .sig
        .inputs
        .iter()
        .filter_map(|fn_arg| match fn_arg {
            FnArg::Receiver(_) => None,
            FnArg::Typed(PatType { pat, .. }) => match &**pat {
                Pat::Ident(PatIdent { ident, .. }) => Some(ident.clone()),
                _ => None,
            },
        })
        .collect()
}

impl<'ast> Visit<'ast> for SelfTailAnalyzer<'_> {
    fn visit_pat_ident(&mut self, pat_ident: &'ast PatIdent) {
        if !self.eligible {
            return;
        }

        if self.is_argument_ident(&pat_ident.ident) {
            self.eligible = false;
            return;
        }

        visit::visit_pat_ident(self, pat_ident);
    }

    fn visit_item_macro(&mut self, item_macro: &'ast ItemMacro) {
        if !self.eligible {
            return;
        }

        if is_tailcall_macro(&item_macro.mac.path) {
            match parse2::<ExprCall>(item_macro.mac.tokens.clone()) {
                Ok(expr_call) => match &*expr_call.func {
                    Expr::Path(ExprPath { path, .. }) if self.is_self_path(path) => {
                        self.saw_self_tailcall = true;
                    }
                    _ => self.eligible = false,
                },
                Err(_) => self.eligible = false,
            }
            return;
        }

        visit::visit_item_macro(self, item_macro);
    }

    fn visit_expr_macro(&mut self, expr_macro: &'ast ExprMacro) {
        if !self.eligible {
            return;
        }

        if is_tailcall_macro(&expr_macro.mac.path) {
            match parse2::<ExprCall>(expr_macro.mac.tokens.clone()) {
                Ok(expr_call) => match &*expr_call.func {
                    Expr::Path(ExprPath { path, .. }) if self.is_self_path(path) => {
                        self.saw_self_tailcall = true;
                    }
                    _ => self.eligible = false,
                },
                Err(_) => self.eligible = false,
            }
            return;
        }

        visit::visit_expr_macro(self, expr_macro);
    }

    fn visit_expr_call(&mut self, expr_call: &'ast ExprCall) {
        if !self.eligible {
            return;
        }

        if let Expr::Path(ExprPath { path, .. }) = &*expr_call.func {
            if self.is_self_path(path) {
                self.eligible = false;
                return;
            }
        }

        visit::visit_expr_call(self, expr_call);
    }

    fn visit_expr_closure(&mut self, _expr_closure: &'ast syn::ExprClosure) {}

    fn visit_item_fn(&mut self, _item_fn: &'ast ItemFn) {}
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::{analyze, is_simple_self_tail_recursive};
    use crate::call_syntax::is_tailcall_macro;

    #[test]
    fn accepts_simple_self_tail_recursion() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn countdown(n: u64) -> u64 {
                if n > 0 {
                    tailcall::call! { countdown(n - 1) }
                } else {
                    0
                }
            }
        };

        assert_eq!(analyze(&item_fn), (true, true));
        assert!(is_simple_self_tail_recursive(&item_fn));
    }

    #[test]
    fn accepts_self_tail_recursion_with_non_recursive_calls() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn scramble_tailcall_go(n: u64, state: u64) -> u64 {
                if n > 0 {
                    tailcall::call! { scramble_tailcall_go(n - 1, scramble_step(state, n)) }
                } else {
                    state
                }
            }
        };

        assert_eq!(analyze(&item_fn), (true, true));
        assert!(is_simple_self_tail_recursive(&item_fn));
    }

    #[test]
    fn rejects_shadowing_parameter_bindings() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn sum_csv_numbers_inner(rest: &[u8], total: u64, current: u64) -> u64 {
                match rest {
                    [digit @ b'0'..=b'9', tail @ ..] => {
                        let current = current * 10 + u64::from(digit - b'0');
                        tailcall::call! { sum_csv_numbers_inner(tail, total, current) }
                    }
                    [] => total + current,
                    [_other, tail @ ..] => {
                        let total = total + current;
                        tailcall::call! { sum_csv_numbers_inner(tail, total, 0) }
                    }
                }
            }
        };

        assert_eq!(analyze(&item_fn), (false, false));
        assert!(!is_simple_self_tail_recursive(&item_fn));
    }

    #[test]
    fn recognizes_tailcall_macro_path() {
        let expr_macro: syn::ExprMacro = parse_quote! {
            tailcall::call! { countdown(n - 1) }
        };

        assert!(is_tailcall_macro(&expr_macro.mac.path));
    }

    #[test]
    fn parses_tailcall_site_as_expression_macro() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn countdown(n: u64) -> u64 {
                if n > 0 {
                    tailcall::call! { countdown(n - 1) }
                } else {
                    0
                }
            }
        };

        let syn::Stmt::Expr(syn::Expr::If(expr_if)) = &item_fn.block.stmts[0] else {
            panic!("expected top-level if expression");
        };
        assert_eq!(expr_if.then_branch.stmts.len(), 1);
        let item_macro = match &expr_if.then_branch.stmts[0] {
            syn::Stmt::Item(syn::Item::Macro(item_macro)) => item_macro,
            _ => panic!("expected tailcall macro item in then branch"),
        };

        assert!(is_tailcall_macro(&item_macro.mac.path));
    }
}
