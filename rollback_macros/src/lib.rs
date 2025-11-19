use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{braced, parse::{Parse, ParseStream}, parse_macro_input, Block, Ident, Result, Token, Type};

struct ViewArg { ident: Ident, ty: Type, is_mut: bool }

fn parse_view_args(input: ParseStream) -> Result<Vec<ViewArg>> {
    let mut args = Vec::new();
    while !input.is_empty() {
        let ident: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty_view: Type = input.parse()?;
        let (ty_inner, is_mut): (Type, bool) = match ty_view {
            Type::Path(ref tp) => {
                let seg = tp.path.segments.last().ok_or_else(|| input.error("expected View/Mut<...>"))?;
                let is_mut = seg.ident == "ViewMut";
                if !(seg.ident == "View" || seg.ident == "ViewMut") { return Err(input.error("expected View or ViewMut")); }
                match &seg.arguments {
                    syn::PathArguments::AngleBracketed(ab) => {
                        match ab.args.first() {
                            Some(syn::GenericArgument::Type(t)) => (t.clone(), is_mut),
                            _ => return Err(input.error("expected View<T>")),
                        }
                    }
                    _ => return Err(input.error("expected View<T>")),
                }
            }
            _ => return Err(input.error("expected View<T>")),
        };
        args.push(ViewArg { ident, ty: ty_inner, is_mut });
        if input.peek(Token![,]) { input.parse::<Token![,]>()?; } else { break; }
    }
    Ok(args)
}

fn parse_type_list_bracketed(input: ParseStream) -> Result<Vec<Type>> {
    let content;
    syn::bracketed!(content in input);
    let mut tys = Vec::new();
    while !content.is_empty() {
        let ty: Type = content.parse()?;
        tys.push(ty);
        if content.peek(Token![,]) { content.parse::<Token![,]>()?; } else { break; }
    }
    Ok(tys)
}

struct SystemInput {
    stage_ident: Ident,
    fn_ident: Ident,
    view_args: Vec<ViewArg>,
    all_types: Vec<Type>,
    none_types: Vec<Type>,
    body: Block,
}

impl Parse for SystemInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let stage_ident: Ident = input.parse()?;
        let outer;
        braced!(outer in input);
        let inner;
        if outer.peek(Ident) {
            let kw: Ident = outer.parse()?;
            if kw == "query" {
                outer.parse::<Token![!]>()?;
                let q; braced!(q in outer); inner = q;
            } else {
                return Err(outer.error("expected query!"));
            }
        } else {
            let q; braced!(q in outer); inner = q;
        }
        inner.parse::<Token![fn]>()?;
        let fn_ident: Ident = inner.parse()?;
        let args_paren; syn::parenthesized!(args_paren in inner);
        let view_args = parse_view_args(&args_paren)?;
        let mut all_types = Vec::new();
        let mut none_types = Vec::new();
        while inner.peek(Ident) {
            let kw: Ident = inner.parse()?;
            if kw == "All" {
                inner.parse::<Token![=]>()?;
                all_types = parse_type_list_bracketed(&inner)?;
            } else if kw == "None" {
                inner.parse::<Token![=]>()?;
                none_types = parse_type_list_bracketed(&inner)?;
            } else { break; }
        }
        let body: Block = inner.parse()?;
        Ok(SystemInput { stage_ident, fn_ident, view_args, all_types, none_types, body })
    }
}

#[proc_macro]
pub fn system(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as SystemInput);

    let stage_ident = parsed.stage_ident;
    let fn_ident = parsed.fn_ident;
    let view_args = parsed.view_args;
    let all_types = parsed.all_types;
    let none_types = parsed.none_types;
    let body = parsed.body;

    let view_idents: Vec<Ident> = view_args.iter().map(|v| v.ident.clone()).collect();
    let view_types: Vec<Type> = view_args.iter().map(|v| v.ty.clone()).collect();

    let all_idents: Vec<Ident> = (0..all_types.len()).map(|i| format_ident!("all_s{}", i+1)).collect();
    let none_idents: Vec<Ident> = (0..none_types.len()).map(|i| format_ident!("none_s{}", i+1)).collect();

    let all_storage_field_idents: Vec<Ident> = (0..all_types.len()).map(|i| format_ident!("all_storage{}", i+1)).collect();
    let none_storage_field_idents: Vec<Ident> = (0..none_types.len()).map(|i| format_ident!("none_storage{}", i+1)).collect();

    let fn_inputs = view_args.iter().map(|va| {
        let vi = &va.ident; let ty = &va.ty;
        if va.is_mut { quote!(#vi: crate::storage::view::ViewMut<#ty>) } else { quote!(#vi: crate::storage::view::View<#ty>) }
    });
    let fn_def = quote! { fn #fn_ident( #(#fn_inputs),* ) #body };

    let none_tuple_types = none_types.iter().map(|t| quote!(&crate::storage::storage::Storage<#t>));
    let all_tuple_types = all_types.iter().map(|t| quote!(&crate::storage::storage::Storage<#t>));
    let view_tuple_types_mixed = view_args.iter().map(|va| {
        let t = &va.ty;
        if va.is_mut { quote!(&mut crate::storage::storage::Storage<#t>) } else { quote!(&crate::storage::storage::Storage<#t>) }
    });
    let mut args_tuple_segs: Vec<proc_macro2::TokenStream> = Vec::new();
    if !none_types.is_empty() { args_tuple_segs.push(quote!( #( #none_tuple_types ),* )); }
    if !all_types.is_empty() { args_tuple_segs.push(quote!( #( #all_tuple_types ),* )); }
    args_tuple_segs.push(quote!( #( #view_tuple_types_mixed ),* ));
    let args_tuple_type = quote!( ( #(#args_tuple_segs),* ) );

    let mut destructure_segs: Vec<proc_macro2::TokenStream> = Vec::new();
    if !none_types.is_empty() { destructure_segs.push(quote!( #( #none_idents ),* )); }
    if !all_types.is_empty() { destructure_segs.push(quote!( #( #all_idents ),* )); }
    destructure_segs.push(quote!( #( #view_idents ),* ));
    let args_destructure = quote!( let ( #(#destructure_segs),* ) = args; );

    let outer_intersections = quote!( #( outer_mask &= #view_idents.root.presence_mask; )* );
    let middle_intersections_views = quote!( #( middle_mask &= unsafe { #view_idents.root.data[oi as usize].assume_init_ref().presence_mask }; )* );
    let inner_intersections_views = quote!( #( inner_mask &= unsafe { #view_idents.root.data[oi as usize].assume_init_ref().data[mi as usize].assume_init_ref().presence_mask }; )* );

    let middle_all = if all_types.is_empty() { quote!() } else {
        let per_all = all_idents.iter().map(|ai| {
            quote! {
                let rp = #ai.root.presence_mask;
                let mut all_mid_single: u128 = u128::MAX;
                if ((rp >> oi) & 1) != 0 {
                    let ab = unsafe { #ai.root.data[oi as usize].assume_init_ref() };
                    all_mid_single &= ab.presence_mask;
                } else { all_mid_single &= 0; }
                all_mid &= all_mid_single;
            }
        });
        quote! { let mut all_mid: u128 = u128::MAX; #(#per_all)* middle_mask &= all_mid; }
    };

    let middle_none = if none_types.is_empty() { quote!() } else {
        let per_none = none_idents.iter().map(|ni| {
            quote! {
                let rp = #ni.root.presence_mask;
                if ((rp >> oi) & 1) != 0 {
                    let nb = unsafe { #ni.root.data[oi as usize].assume_init_ref() };
                    none_mid |= nb.absence_mask;
                }
            }
        });
        quote! { let mut none_mid: u128 = 0; #(#per_none)* middle_mask &= !none_mid; }
    };

    let inner_all = if all_types.is_empty() { quote!() } else {
        let per_all = all_idents.iter().map(|ai| {
            quote! {
                let ab = unsafe { #ai.root.data[oi as usize].assume_init_ref() };
                let mp = ab.presence_mask;
                let mut all_in_single: u128 = u128::MAX;
                if ((mp >> mi) & 1) != 0 {
                    let ib = unsafe { ab.data[mi as usize].assume_init_ref() };
                    all_in_single &= ib.presence_mask;
                } else { all_in_single &= 0; }
                all_in &= all_in_single;
            }
        });
        quote! { let mut all_in: u128 = u128::MAX; #(#per_all)* inner_mask &= all_in; }
    };

    let inner_none = if none_types.is_empty() { quote!() } else {
        let per_none = none_idents.iter().map(|ni| {
            quote! {
                let nb = unsafe { #ni.root.data[oi as usize].assume_init_ref() };
                let mp = nb.presence_mask;
                if ((mp >> mi) & 1) != 0 {
                    let ib = unsafe { nb.data[mi as usize].assume_init_ref() };
                    none_in |= ib.absence_mask;
                }
            }
        });
        quote! { let mut none_in: u128 = 0; #(#per_none)* inner_mask &= !none_in; }
    };

    let call_views = {
        let view_slices = view_args.iter().map(|va| {
            let vi = &va.ident; let ty = &va.ty;
            if va.is_mut {
                quote! {
                    crate::storage::view::ViewMut {
                        data: unsafe {
                            let ptr = #vi.root.data[oi as usize]
                                .assume_init_ref()
                                .data[mi as usize]
                                .assume_init_ref()
                                .data
                                .as_ptr() as *mut #ty;
                            std::slice::from_raw_parts_mut(ptr.add(start as usize), run as usize)
                        }
                    }
                }
            } else {
                quote! {
                    crate::storage::view::View {
                        data: unsafe {
                            let ptr = #vi.root.data[oi as usize]
                                .assume_init_ref()
                                .data[mi as usize]
                                .assume_init_ref()
                                .data
                                .as_ptr() as *const #ty;
                            std::slice::from_raw_parts(ptr.add(start as usize), run as usize)
                        }
                    }
                }
            }
        });
        quote!( self( #( #view_slices ),* ); )
    };

    let fn_arg_types = view_args.iter().map(|va| {
        let t = &va.ty;
        if va.is_mut { quote!(crate::storage::view::ViewMut<#t>) } else { quote!(crate::storage::view::View<#t>) }
    });
    let fn_arg_types_run = view_args.iter().map(|va| {
        let t = &va.ty;
        if va.is_mut { quote!(crate::storage::view::ViewMut<#t>) } else { quote!(crate::storage::view::View<#t>) }
    });

    let query_impl = quote! {
        impl crate::system::Query<#args_tuple_type> for fn( #( #fn_arg_types ),* ) {
            fn run(&self, args: #args_tuple_type ) {
                #args_destructure
                let mut outer_mask: u128 = u128::MAX;
                #outer_intersections
                while outer_mask != 0 {
                    let oi = outer_mask.trailing_zeros();
                    let mut middle_mask: u128 = u128::MAX;
                    #middle_intersections_views
                    #middle_all
                    #middle_none
                    while middle_mask != 0 {
                        let mi = middle_mask.trailing_zeros();
                        let mut inner_mask: u128 = u128::MAX;
                        #inner_intersections_views
                        #inner_all
                        #inner_none
                        while inner_mask != 0 {
                            let start = inner_mask.trailing_zeros();
                            let run = (inner_mask >> start).trailing_ones();
                            #call_views
                            let range_mask = if run == 128 { u128::MAX } else { ((1u128 << run) - 1) << start };
                            inner_mask &= !range_mask;
                        }
                        middle_mask &= !(1u128 << mi);
                    }
                    outer_mask &= !(1u128 << oi);
                }
            }
        }
    };

    let struct_fields_all = all_types.iter().enumerate().map(|(i, t)| {
        let id = &all_storage_field_idents[i];
        quote!( pub #id: std::rc::Rc<std::cell::RefCell<crate::storage::storage::Storage<#t>>> , )
    });
    let struct_fields_none = none_types.iter().enumerate().map(|(i, t)| {
        let id = &none_storage_field_idents[i];
        quote!( pub #id: std::rc::Rc<std::cell::RefCell<crate::storage::storage::Storage<#t>>> , )
    });
    let struct_fields_views = view_types.iter().enumerate().map(|(i, t)| {
        let id = &view_idents[i];
        quote!( pub #id: std::rc::Rc<std::cell::RefCell<crate::storage::storage::Storage<#t>>> , )
    });

    let run_args_none = none_storage_field_idents.iter().map(|id| quote!(&*self.#id.borrow()));
    let run_args_all = all_storage_field_idents.iter().map(|id| quote!(&*self.#id.borrow()));
    let run_args_views_mixed = view_args.iter().map(|va| {
        let id = &va.ident; if va.is_mut { quote!(&mut *self.#id.borrow_mut()) } else { quote!(&*self.#id.borrow()) }
    });
    let mut run_args_segs: Vec<proc_macro2::TokenStream> = Vec::new();
    if !none_types.is_empty() { run_args_segs.push(quote!( #( #run_args_none ),* )); }
    if !all_types.is_empty() { run_args_segs.push(quote!( #( #run_args_all ),* )); }
    run_args_segs.push(quote!( #( #run_args_views_mixed ),* ));

    let create_fields_none = none_types.iter().enumerate().map(|(i, t)| {
        let id = &none_storage_field_idents[i]; quote!( #id: world.get::<#t>() )
    });
    let create_fields_all = all_types.iter().enumerate().map(|(i, t)| {
        let id = &all_storage_field_idents[i]; quote!( #id: world.get::<#t>() )
    });
    let create_fields_views = view_types.iter().enumerate().map(|(i, t)| {
        let id = &view_idents[i]; quote!( #id: world.get::<#t>() )
    });
    let mut create_segs: Vec<proc_macro2::TokenStream> = Vec::new();
    if !none_types.is_empty() { create_segs.push(quote!( #( #create_fields_none ),* )); }
    if !all_types.is_empty() { create_segs.push(quote!( #( #create_fields_all ),* )); }
    create_segs.push(quote!( #( #create_fields_views ),* ));

    let reads_types = none_types.iter().map(|t| quote!( std::any::TypeId::of::<#t>() ));
    let reads_types_all = all_types.iter().map(|t| quote!( std::any::TypeId::of::<#t>() ));
    let reads_types_views = view_args.iter().filter(|va| !va.is_mut).map(|va| {
        let t = &va.ty; quote!( std::any::TypeId::of::<#t>() )
    });
    let writes_types_views = view_args.iter().filter(|va| va.is_mut).map(|va| {
        let t = &va.ty; quote!( std::any::TypeId::of::<#t>() )
    });
    let mut reads_segs: Vec<proc_macro2::TokenStream> = Vec::new();
    if !none_types.is_empty() { reads_segs.push(quote!( #( #reads_types ),* )); }
    if !all_types.is_empty() { reads_segs.push(quote!( #( #reads_types_all ),* )); }
    reads_segs.push(quote!( #( #reads_types_views ),* ));

    let expanded = quote! {
        #fn_def
        pub struct #stage_ident { #( #struct_fields_none )* #( #struct_fields_all )* #( #struct_fields_views )* }
        impl crate::scheduler::pipeline::PipelineStage for #stage_ident {
            fn run(&self) {
                let sys: fn( #( #fn_arg_types_run ),* ) = #fn_ident;
                crate::system::Query::run(&sys, ( #(#run_args_segs),* ));
            }
            fn create(world: &mut crate::world::World) -> Self {
                Self { #(#create_segs),* }
            }
            fn reads(&self) -> &'static [std::any::TypeId] {
                static READS: &[std::any::TypeId] = &[ #(#reads_segs),* ];
                READS
            }
            fn writes(&self) -> &'static [std::any::TypeId] {
                static WRITES: &[std::any::TypeId] = &[ #( #writes_types_views ),* ];
                WRITES
            }
        }
        #query_impl
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn variadic_system(input: TokenStream) -> TokenStream {
    system(input)
}
