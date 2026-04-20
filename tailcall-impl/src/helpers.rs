use syn::{parse_quote, Expr, FnArg, Ident, Pat, PatIdent, PatType, Path, Signature};

pub trait FunctionArgs {
    fn argument_exprs(&self) -> Vec<Expr>;
}

impl FunctionArgs for Signature {
    fn argument_exprs(&self) -> Vec<Expr> {
        self.inputs.iter().map(argument_expr).collect()
    }
}

fn argument_expr(fn_arg: &FnArg) -> Expr {
    match fn_arg {
        FnArg::Typed(PatType { pat, .. }) => match &**pat {
            Pat::Ident(PatIdent {
                attrs,
                by_ref: None,
                ident,
                subpat: None,
                ..
            }) if attrs.is_empty() => parse_quote! { #ident },
            _ => unimplemented!(
                "tail recursion for functions with more than simple patterns in the argument list is not supported"
            ),
        },
        FnArg::Receiver(_) => {
            unimplemented!("tail recursion in methods (functions with `self` in the arguments list) is not supported")
        }
    }
}

pub fn helper_ident(fn_name: &Ident) -> Ident {
    Ident::new(&format!("__tailcall_build_{}", fn_name), fn_name.span())
}

pub fn is_tailcall_macro(path: &Path) -> bool {
    path.segments
        .last()
        .map_or(false, |segment| segment.ident == "call")
}

pub fn helper_path_for(path: &Path) -> Path {
    let mut helper_path = path.clone();
    let last_segment = helper_path
        .segments
        .last_mut()
        .expect("tailcall::call! requires a function path");

    last_segment.ident = helper_ident(&last_segment.ident);

    helper_path
}
