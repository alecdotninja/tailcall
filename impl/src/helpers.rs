use syn::{parse_quote, Expr, FnArg, Pat, PatIdent, PatType, Signature};

pub trait RewriteForBindLater {
    fn bind_later(&mut self) -> Vec<(Pat, Expr)>;
}

impl RewriteForBindLater for Pat {
    fn bind_later(&mut self) -> Vec<(Pat, Expr)> {
        match self {
            Pat::Ident(PatIdent { attrs, subpat: None, mutability, ident, .. }) if attrs.is_empty() => {
                vec![
                    (
                        Pat::Ident(PatIdent {
                            attrs: vec![],
                            subpat: None,
                            by_ref: None,
                            mutability: mutability.take(),
                            ident: ident.clone(),
                        }),
                        parse_quote! { #ident },
                    )
                ]
            },
            _ => unimplemented!("tail recursion for functions with more than simple patterns in the argument list is not supported"),
        }
    }
}

impl RewriteForBindLater for FnArg {
    fn bind_later(&mut self) -> Vec<(Pat, Expr)> {
        match self {
            FnArg::Typed(PatType { pat, .. }) => pat.bind_later(),
            FnArg::Receiver(_) => unimplemented!("tail recursion in methods (functions with `self` in the arguments list) is not supported"),
        }
    }
}

impl RewriteForBindLater for Signature {
    fn bind_later(&mut self) -> Vec<(Pat, Expr)> {
        self.inputs
            .iter_mut()
            .flat_map(|fn_arg| fn_arg.bind_later())
            .collect()
    }
}

pub trait Binding {
    fn tuple_pat(&self) -> Pat;
    fn tuple_expr(&self) -> Expr;
}

impl Binding for Vec<(Pat, Expr)> {
    fn tuple_pat(&self) -> Pat {
        let pats: Vec<&Pat> = self.iter().map(|(pat, _expr)| pat).collect();

        parse_quote! { (#(#pats),*) }
    }

    fn tuple_expr(&self) -> Expr {
        let exprs: Vec<&Expr> = self.iter().map(|(_pat, expr)| expr).collect();

        parse_quote! { (#(#exprs),*) }
    }
}
