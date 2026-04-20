use proc_macro2::Span;
use syn::{
    parse_quote, Error, Expr, ExprCall, ExprMethodCall, ExprPath, FnArg, GenericParam, Ident,
    Lifetime, Pat, PatIdent, PatType, Path, Receiver, ReturnType, Signature, Type, TypeReference,
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
        FnArg::Receiver(_) => Ok(parse_quote! { self }),
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

pub fn helper_method_call_tokens(expr_method_call: &ExprMethodCall) -> Result<proc_macro2::TokenStream, Error> {
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

pub fn method_helper_signature(sig: &Signature) -> Result<Signature, Error> {
    let mut helper_sig = sig.clone();
    let output_ty = output_type(&sig.output);
    let tailcall_lifetime = Lifetime::new("'tailcall", Span::call_site());

    helper_sig.ident = helper_ident(&sig.ident);
    helper_sig
        .generics
        .params
        .push(parse_quote!(#tailcall_lifetime));
    rewrite_method_inputs(&mut helper_sig.inputs, &tailcall_lifetime)?;
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

    Ok(helper_sig)
}

fn rewrite_method_inputs(
    inputs: &mut syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
    lifetime: &Lifetime,
) -> Result<(), Error> {
    for input in inputs {
        match input {
            FnArg::Receiver(receiver) => rewrite_receiver(receiver, lifetime),
            FnArg::Typed(pat_type) => rewrite_elided_lifetimes_in_type(pat_type.ty.as_mut(), lifetime),
        }
    }

    Ok(())
}

fn rewrite_receiver(receiver: &mut Receiver, lifetime: &Lifetime) {
    if let Some((_and_token, receiver_lifetime)) = &mut receiver.reference {
        if receiver_lifetime.is_none() {
            *receiver_lifetime = Some(lifetime.clone());
        }
    }
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
