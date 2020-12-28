#![feature(proc_macro_diagnostic)]

use proc_macro::{Span, TokenStream};
use quote::quote;

#[proc_macro_attribute]
pub fn main(_: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    let ret = input.sig.output;
    let name = &input.sig.ident;
    let inputs = &input.sig.inputs;
    let body = &input.block;
    let attrs = &input.attrs;

    if !inputs.is_empty() {
        let msg = "The main function cannot accept arguments";
        return syn::Error::new_spanned(&input.sig.inputs, msg)
            .to_compile_error()
            .into();
    }

    // FIXME: For some reason comparing syn::ReturnType doesnt work coz PartialEq isnt implemented
    //        for these, even tho the docs say that they are.
    match ret {
        syn::ReturnType::Default => {}
        _ => {
            let msg = "The main function should not return anything";
            return syn::Error::new_spanned(&input.sig.inputs, msg)
                .to_compile_error()
                .into();
        }
    }

    let result = quote! {
        bootloader::entry_point!(#name);
        #(#attrs)*
        fn #name(boot_info: &'static bootloader::BootInfo) -> ! {
            println!("Booting... Standby...");
            vallicks::init(boot_info);
            println!("Booted in {}ms", timer::get_milis());

            // if we are in testing mode we run all the tests and halt
            if cfg!(test) {
                #[cfg(test)]
                test_main();
                halt();
            }

            // We spawn the old main inside a closure as a separate thread
            #body

            // We attempt to join this thread, if the thread panics we send a ErrorCode downstream
            // to qemu
            halt();
        }
    };

    result.into()
}

#[proc_macro_attribute]
pub fn unittest(_: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    let ret = input.sig.output;
    let name = &input.sig.ident;
    let inputs = &input.sig.inputs;
    let body = &input.block;
    let attrs = &input.attrs;

    if !inputs.is_empty() {
        let msg = "The main function cannot accept arguments";
        return syn::Error::new_spanned(&input.sig.inputs, msg)
            .to_compile_error()
            .into();
    }

    // FIXME: For some reason comparing syn::ReturnType doesnt work coz PartialEq isnt implemented
    //        for these, even tho the docs say that they are.
    match ret {
        syn::ReturnType::Default => {}
        _ => {
            let msg = "The main function should not return anything";
            return syn::Error::new_spanned(&input.sig.inputs, msg)
                .to_compile_error()
                .into();
        }
    }

    let as_text = format!("==> {}", name.to_string());

    let result = quote! {
        #(#attrs)*
        #[test_case]
        fn #name() {
            use crate::uprint;
            uprint!(#as_text);
            #body
            uprint!("   [OK]\n");
        }
    };

    result.into()
}

#[proc_macro]
pub fn compile_warning(input: TokenStream) -> TokenStream {
    Span::call_site().warning(input.to_string()).emit();
    TokenStream::new()
}
