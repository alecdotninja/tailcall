use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse2, parse_quote, Error, Expr, ExprCall, ExprMethodCall, ExprPath, Path};

use crate::naming::helper_ident;

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

    Error::new(
        Span::call_site(),
        "tailcall::call! expects either `path(args...)` or `self.method(args...)`",
    )
    .to_compile_error()
}

pub fn helper_path_from_call(expr_call: &ExprCall) -> Result<Path, Error> {
    match &*expr_call.func {
        Expr::Path(ExprPath { path, .. }) => Ok(helper_path_for(path)),
        func => Err(Error::new_spanned(
            func,
            "tailcall::call! expects a direct function path like `foo(...)` or `module::foo(...)`",
        )),
    }
}

pub fn helper_method_call_tokens(expr_method_call: &ExprMethodCall) -> Result<TokenStream, Error> {
    if !matches!(
        &*expr_method_call.receiver,
        Expr::Path(ExprPath { path, .. }) if path.is_ident("self")
    ) {
        return Err(Error::new_spanned(
            &expr_method_call.receiver,
            "tailcall::call! only supports method syntax on `self`; use `Self::method(self, ...)` for other receivers",
        ));
    }

    let helper = helper_ident(&expr_method_call.method);
    let args = &expr_method_call.args;

    Ok(parse_quote! { self.#helper(#args) })
}

pub fn is_tailcall_macro(path: &Path) -> bool {
    match path.segments.last() {
        Some(last) if last.ident == "call" => {}
        _ => return false,
    }

    match path.segments.len() {
        1 => true,
        2 => path.segments[0].ident == "tailcall",
        _ => false,
    }
}

fn helper_path_for(path: &Path) -> Path {
    let mut helper_path = path.clone();
    let last_segment = helper_path
        .segments
        .last_mut()
        .expect("function path should have at least one segment");

    last_segment.ident = helper_ident(&last_segment.ident);
    helper_path
}
