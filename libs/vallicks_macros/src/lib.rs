use proc_macro::TokenStream;
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

            // We spawn the old main inside a closure as a separate thread
            thread::spawn(||{
                #body
            });

            // NOTE: Do not add anymore code after the main body has been called.
            halt();
        }
    };

    result.into()
}
