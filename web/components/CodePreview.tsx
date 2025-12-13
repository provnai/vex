"use client";

import { motion } from "framer-motion";
import { Copy, Check, Terminal } from "lucide-react";
import { useState } from "react";

const codeExample = `use vex_core::{Agent, Context};
use vex_llm::OpenAIProvider;
use vex_persist::SqliteBackend;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize with cryptographic audit trail
    let agent = Agent::builder()
        .name("analyst-agent")
        .provider(OpenAIProvider::new())
        .persistence(SqliteBackend::encrypted("./data"))
        .with_merkle_audit()
        .build()
        .await?;

    // Execute with full traceability
    let response = agent
        .execute("Analyze Q4 market trends")
        .with_context(Context::temporal(Duration::hours(24)))
        .with_adversarial_verification()
        .await?;

    // Every action is cryptographically verified
    assert!(agent.audit_log().verify_integrity());
    
    Ok(())
}`;

export function CodePreview() {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(codeExample);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <section className="py-32 px-4 relative overflow-hidden">
      {/* Background decoration */}
      <div className="absolute inset-0 bg-linear-to-b from-transparent via-zinc-900/50 to-transparent" />
      
      <div className="max-w-5xl mx-auto relative">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="text-center mb-12 space-y-4"
        >
          <span className="text-primary text-sm font-medium uppercase tracking-wider">Developer Experience</span>
          <h2 className="text-4xl md:text-5xl font-bold text-white">
            Clean, <span className="text-primary">Intuitive</span> APIs
          </h2>
          <p className="text-zinc-400 max-w-2xl mx-auto text-lg">
            Production-ready code that feels native to Rust. No boilerplate, no magic—just performant, verifiable agent infrastructure.
          </p>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20, scale: 0.95 }}
          whileInView={{ opacity: 1, y: 0, scale: 1 }}
          viewport={{ once: true }}
          transition={{ delay: 0.2 }}
          className="relative group"
        >
          {/* Glow effect */}
          <div className="absolute -inset-1 bg-linear-to-r from-primary/20 via-purple-500/20 to-primary/20 rounded-2xl blur-xl opacity-0 group-hover:opacity-100 transition-opacity duration-500" />
          
          {/* Window chrome */}
          <div className="absolute -inset-px bg-linear-to-b from-zinc-700/50 to-transparent rounded-2xl opacity-0 group-hover:opacity-100 transition-opacity" />
          
          <div className="relative bg-zinc-900 border border-zinc-800 rounded-2xl overflow-hidden shadow-2xl">
            {/* Title bar */}
            <div className="flex items-center justify-between px-4 py-3 bg-zinc-900/80 border-b border-zinc-800">
              <div className="flex items-center gap-2">
                <motion.div whileHover={{ scale: 1.2 }} className="w-3 h-3 rounded-full bg-red-500/80 cursor-pointer" />
                <motion.div whileHover={{ scale: 1.2 }} className="w-3 h-3 rounded-full bg-yellow-500/80 cursor-pointer" />
                <motion.div whileHover={{ scale: 1.2 }} className="w-3 h-3 rounded-full bg-green-500/80 cursor-pointer" />
              </div>
              <div className="flex items-center gap-2 text-xs text-zinc-500 font-mono">
                <Terminal className="w-3 h-3" />
                main.rs
              </div>
              <motion.button
                onClick={handleCopy}
                whileHover={{ scale: 1.1 }}
                whileTap={{ scale: 0.95 }}
                className="p-2 hover:bg-zinc-800 rounded-lg transition-colors"
              >
                {copied ? (
                  <Check className="w-4 h-4 text-green-400" />
                ) : (
                  <Copy className="w-4 h-4 text-zinc-500" />
                )}
              </motion.button>
            </div>

            {/* Code content */}
            <div className="p-6 overflow-x-auto">
              <pre className="text-sm font-mono leading-relaxed">
                <code>
                  {codeExample.split("\n").map((line, i) => (
                    <motion.div 
                      key={i} 
                      className="table-row"
                      initial={{ opacity: 0, x: -10 }}
                      whileInView={{ opacity: 1, x: 0 }}
                      viewport={{ once: true }}
                      transition={{ delay: i * 0.02 }}
                    >
                      <span className="table-cell pr-4 text-zinc-600 text-right select-none w-8">
                        {i + 1}
                      </span>
                      <span className="table-cell">
                        <SyntaxHighlight line={line} />
                      </span>
                    </motion.div>
                  ))}
                </code>
              </pre>
            </div>
          </div>
        </motion.div>

        {/* Feature pills below code */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ delay: 0.4 }}
          className="flex flex-wrap justify-center gap-3 mt-8"
        >
          {["Type Safe", "Async Native", "Zero Unsafe", "Fully Documented"].map((tag, i) => (
            <motion.span
              key={tag}
              initial={{ opacity: 0, scale: 0.9 }}
              whileInView={{ opacity: 1, scale: 1 }}
              viewport={{ once: true }}
              transition={{ delay: 0.5 + i * 0.1 }}
              className="px-4 py-2 bg-zinc-900/80 border border-zinc-800 rounded-full text-zinc-400 text-sm"
            >
              {tag}
            </motion.span>
          ))}
        </motion.div>
      </div>
    </section>
  );
}

function SyntaxHighlight({ line }: { line: string }) {
  // Simple syntax highlighting
  const highlighted = line
    .replace(/(use|async|fn|let|await|Ok|assert!)/g, '<span class="text-purple-400">$1</span>')
    .replace(/(".*?")/g, '<span class="text-green-400">$1</span>')
    .replace(/(\/\/.*)/g, '<span class="text-zinc-500">$1</span>')
    .replace(/(\.\w+\()/g, '<span class="text-blue-400">$1</span>')
    .replace(/(Agent|Context|OpenAIProvider|SqliteBackend|Duration|Result)/g, '<span class="text-yellow-400">$1</span>')
    .replace(/(main|builder|name|provider|persistence|encrypted|with_merkle_audit|build|execute|with_context|with_adversarial_verification|temporal|hours|audit_log|verify_integrity)/g, '<span class="text-blue-300">$1</span>');

  return <span dangerouslySetInnerHTML={{ __html: highlighted }} />;
}
