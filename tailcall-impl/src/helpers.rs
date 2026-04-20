use proc_macro2::Span;
use syn::{
    parse_quote, Error, Expr, ExprCall, ExprPath, FnArg, GenericParam, Ident, Lifetime, Pat,
    PatIdent, PatType, Path, ReturnType, Signature, Type, TypeReference,
};

pub fn helper_ident(fn_name: &Ident) -> Ident {
    Ident::new(&format!("__tailcall_build_{}", fn_name), fn_name.span())
}

pub fn output_type(output: &ReturnType) -> Type {
    match output {
        ReturnType::Default => parse_quote! { () },
        ReturnType::Type(_, ty) => (**ty).clone(),
    }
}

pub fn helper_signature(sig: &Signature) -> Signature {
    let mut helper_sig = sig.clone();
    let output_ty = output_type(&sig.output);
    let tailcall_lifetime = Lifetime::new("'tailcall", Span::call_site());

    helper_sig.ident = helper_ident(&sig.ident);
    helper_sig
        .generics
        .params
        .push(parse_quote!(#tailcall_lifetime));
    rewrite_elided_lifetimes_in_inputs(&mut helper_sig.inputs, &tailcall_lifetime);
    helper_sig.output = parse_quote! { -> tailcall::trampoline::Action<#tailcall_lifetime, #output_ty> };

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

pub fn function_argument_exprs(sig: &Signature) -> Result<Vec<Expr>, Error> {
    sig.inputs.iter().map(argument_expr).collect()
}

fn argument_expr(fn_arg: &FnArg) -> Result<Expr, Error> {
    match fn_arg {
        FnArg::Typed(PatType { pat, .. }) => match &**pat {
            Pat::Ident(PatIdent {
                attrs,
                by_ref: None,
                ident,
                subpat: None,
                ..
            }) if attrs.is_empty() => Ok(parse_quote! { #ident }),
            pat => Err(Error::new_spanned(
                pat,
                "#[tailcall] only supports simple identifier arguments",
            )),
        },
        FnArg::Receiver(receiver) => Err(Error::new_spanned(
            receiver,
            "#[tailcall] does not support methods or `self` receivers",
        )),
    }
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

fn rewrite_elided_lifetimes_in_inputs(
    inputs: &mut syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
    lifetime: &Lifetime,
) {
    for input in inputs {
        if let FnArg::Typed(PatType { ty, .. }) = input {
            rewrite_elided_lifetimes_in_type(ty.as_mut(), lifetime);
        }
    }
}

fn rewrite_elided_lifetimes_in_type(ty: &mut Type, lifetime: &Lifetime) {
    match ty {
        Type::Reference(TypeReference {
            lifetime: ty_lifetime,
            elem,
            ..
        }) => {
            if ty_lifetime.is_none() {
                *ty_lifetime = Some(lifetime.clone());
            }
            rewrite_elided_lifetimes_in_type(elem.as_mut(), lifetime);
        }
        Type::Slice(type_slice) => rewrite_elided_lifetimes_in_type(type_slice.elem.as_mut(), lifetime),
        Type::Array(type_array) => rewrite_elided_lifetimes_in_type(type_array.elem.as_mut(), lifetime),
        Type::Tuple(type_tuple) => {
            for elem in &mut type_tuple.elems {
                rewrite_elided_lifetimes_in_type(elem, lifetime);
            }
        }
        Type::Paren(type_paren) => rewrite_elided_lifetimes_in_type(type_paren.elem.as_mut(), lifetime),
        Type::Group(type_group) => rewrite_elided_lifetimes_in_type(type_group.elem.as_mut(), lifetime),
        _ => {}
    }
}
