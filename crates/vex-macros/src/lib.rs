extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, ItemFn};

/// Auto-implements the `Job` trait for a struct.
///
/// Usage:
/// ```ignore
/// #[derive(VexJob)]
/// struct MyJob { ... }
/// ```
#[proc_macro_derive(VexJob, attributes(job))]
pub fn derive_vex_job(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Use struct name as job name (simplified - no attribute parsing)
    let job_name = name.to_string().to_lowercase();
    let max_retries = 3u32;

    let expanded = quote! {
        #[async_trait::async_trait]
        impl vex_queue::job::Job for #name {
            fn name(&self) -> &str {
                #job_name
            }

            async fn execute(&mut self) -> vex_queue::job::JobResult {
                self.run().await
            }

            fn max_retries(&self) -> u32 {
                #max_retries
            }

            fn backoff_strategy(&self) -> vex_queue::job::BackoffStrategy {
                vex_queue::job::BackoffStrategy::Exponential {
                    initial_secs: 1,
                    multiplier: 2.0
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Generates a ToolDefinition constant for an LLM tool function.
///
/// Usage:
/// ```ignore
/// #[vex_tool]
/// fn web_search(query: String) -> String { ... }
/// ```
///
/// Generates a `WEB_SEARCH_TOOL` constant.
#[proc_macro_attribute]
pub fn vex_tool(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let tool_name = fn_name.to_string();
    let tool_desc = "Auto-generated tool"; // Can be improved by parsing doc comments

    let mut props_map = std::collections::HashMap::new();
    let mut req_list = Vec::new();

    for arg in &input.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = arg {
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                let arg_name = pat_ident.ident.to_string();
                let arg_type = &*pat_type.ty;

                let (json_type, is_optional) = match get_json_type_static(arg_type) {
                    Some((t, opt)) => (t, opt),
                    None => ("string", false),
                };

                props_map.insert(arg_name.clone(), json_type);
                if !is_optional {
                    req_list.push(arg_name);
                }
            }
        }
    }

    let parameters = if props_map.is_empty() {
        "{}".to_string()
    } else {
        let mut props_vec: Vec<_> = props_map.iter().collect();
        props_vec.sort_by_key(|a| a.0); // Sort for deterministic output

        let props_str = props_vec
            .iter()
            .map(|(k, v)| format!("\"{}\":{{\"type\":\"{}\"}}", k, v))
            .collect::<Vec<_>>()
            .join(",");

        let mut req_vec = req_list.clone();
        req_vec.sort();

        let req_str = req_vec
            .iter()
            .map(|k| format!("\"{}\"", k))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"type\":\"object\",\"properties\":{{{}}},\"required\":[{}]}}",
            props_str, req_str
        )
    };

    let const_name = format_ident!("{}_TOOL", tool_name.to_uppercase());

    let expanded = quote! {
        #input

        pub const #const_name: vex_llm::ToolDefinition = vex_llm::ToolDefinition {
            name: #tool_name,
            description: #tool_desc,
            parameters: #parameters,
        };
    };

    TokenStream::from(expanded)
}

fn get_json_type_static(ty: &syn::Type) -> Option<(&'static str, bool)> {
    match ty {
        syn::Type::Path(tp) => {
            let last = tp.path.segments.last()?;
            let ident = last.ident.to_string();
            match ident.as_str() {
                "String" | "str" => Some(("string", false)),
                "i32" | "i64" | "u32" | "u64" | "isize" | "usize" => Some(("integer", false)),
                "f32" | "f64" => Some(("number", false)),
                "bool" => Some(("boolean", false)),
                "Option" => {
                    if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            let (inner_type, _) = get_json_type_static(inner)?;
                            return Some((inner_type, true));
                        }
                    }
                    None
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Instruments an agent function with tracing.
///
/// Usage:
/// ```ignore
/// #[instrument_agent]
/// async fn think(&self) { ... }
/// ```
#[proc_macro_attribute]
pub fn instrument_agent(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let block = &input.block;
    let sig = &input.sig;
    let vis = &input.vis;
    let attrs = &input.attrs;

    let expanded = quote! {
        #(#attrs)*
        #vis #sig {
            let span = tracing::info_span!(
                stringify!(#fn_name),
                agent_id = %self.id,
                generation = %self.generation
            );
            let _enter = span.enter();
            #block
        }
    };

    TokenStream::from(expanded)
}
