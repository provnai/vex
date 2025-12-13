extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{parse_macro_input, DeriveInput, ItemFn, AttributeArgs, Meta, NestedMeta, Lit, Item};

/// Auto-implements the `Job` trait for a struct.
/// 
/// Usage:
/// ```
/// #[derive(VexJob)]
/// #[job(name = "my_job", retries = 3, backoff = "exponential")]
/// struct MyJob { ... }
/// ```
#[proc_macro_derive(VexJob, attributes(job))]
pub fn derive_vex_job(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    
    // Default values
    let mut job_name = name.to_string().to_lowercase();
    let mut max_retries = 3u32;
    let mut backoff_type = "exponential".to_string();

    // Parse attributes
    for attr in input.attrs {
        if attr.path.is_ident("job") {
             if let Ok(Meta::List(list)) = attr.parse_meta() {
                for nested in list.nested {
                    if let NestedMeta::Meta(Meta::NameValue(nv)) = nested {
                        if nv.path.is_ident("name") {
                            if let Lit::Str(lit) = nv.lit {
                                job_name = lit.value();
                            }
                        } else if nv.path.is_ident("retries") {
                            if let Lit::Int(lit) = nv.lit {
                                max_retries = lit.base10_parse().unwrap_or(3);
                            }
                        } else if nv.path.is_ident("backoff") {
                            if let Lit::Str(lit) = nv.lit {
                                backoff_type = lit.value();
                            }
                        }
                    }
                }
             }
        }
    }

    let backoff_expr = match backoff_type.as_str() {
        "constant" => quote! { vex_queue::job::BackoffStrategy::Constant { secs: 5 } },
        _ => quote! { vex_queue::job::BackoffStrategy::Exponential { initial_secs: 1, multiplier: 2.0 } },
    };

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
                #backoff_expr
            }
        }
    };

    TokenStream::from(expanded)
}

/// Generates a JSON Schema for an LLM tool function.
/// 
/// Usage:
/// ```
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
    let tool_desc = "Auto-generated tool"; // Ideally parse doc comments
    
    // Create a CONST name (e.g., search -> SEARCH_TOOL)
    let const_name = format_ident!("{}_TOOL", tool_name.to_uppercase());

    let expanded = quote! {
        #input

        pub const #const_name: vex_llm::ToolDefinition = vex_llm::ToolDefinition {
            name: #tool_name,
            description: #tool_desc,
            parameters: "{}", // TODO: Reflection on input types
        };
    };

    TokenStream::from(expanded)
}

/// Instruments an agent function with OpenTelemetry tracing.
/// 
/// Usage:
/// ```
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

    let expanded = quote! {
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
