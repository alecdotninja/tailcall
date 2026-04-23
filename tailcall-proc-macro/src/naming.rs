use syn::Ident;

pub fn helper_ident(fn_name: &Ident) -> Ident {
    Ident::new(
        &format!("__tailcall_build_{}_thunk", fn_name),
        fn_name.span(),
    )
}
