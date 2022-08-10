use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn struct_copy(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let _ = input;
    unimplemented!()
}
