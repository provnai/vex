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
    let tool_desc = "Auto-generated tool";

    let const_name = format_ident!("{}_TOOL", tool_name.to_uppercase());

    let expanded = quote! {
        #input

        pub const #const_name: vex_llm::ToolDefinition = vex_llm::ToolDefinition {
            name: #tool_name,
            description: #tool_desc,
            parameters: "{}",
        };
    };

    TokenStream::from(expanded)
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
