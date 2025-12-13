"use client";

import { motion, AnimatePresence } from "framer-motion";
import { useState } from "react";
import { Plus, Minus } from "lucide-react";

const faqs = [
  {
    question: "What exactly is included in the acquisition?",
    answer: "The acquisition includes the complete VEX codebase (15,000+ lines of Rust), all intellectual property rights, trademarks, documentation, benchmarks, test suites, and CI/CD configurations. You receive full exclusive ownership with no ongoing royalties or revenue share requirements.",
  },
  {
    question: "Is the codebase production-ready?",
    answer: "Yes. VEX has been designed for production use from day one. It features 94% test coverage, comprehensive error handling, zero unsafe Rust blocks, and has been benchmarked at 550,000+ operations per second. The modular architecture allows for incremental adoption.",
  },
  {
    question: "Are there any third-party license restrictions?",
    answer: "No. All dependencies use permissive licenses (MIT, Apache 2.0) that allow commercial use without restriction. We can provide a complete license audit as part of the due diligence process.",
  },
  {
    question: "What's the process for acquisition?",
    answer: "1) Initial inquiry via email, 2) NDA signing for detailed discussions, 3) Technical due diligence with codebase access, 4) Term sheet and negotiation, 5) Final agreement and IP transfer. We can typically complete the process within 2-4 weeks.",
  },
  {
    question: "Is knowledge transfer available?",
    answer: "Yes, we offer optional consultation packages ranging from documentation handoff to full architecture walkthroughs with the development team. This can include code reviews, integration support, and best practices for your specific use case.",
  },
  {
    question: "Can I see the code before committing?",
    answer: "Absolutely. After signing an NDA, we provide full access to the private repository for technical due diligence. You'll be able to review the code quality, run tests, and evaluate the architecture with your team.",
  },
  {
    question: "What LLM providers are supported?",
    answer: "VEX includes production integrations with OpenAI (GPT-4, GPT-3.5), Ollama (local models), and DeepSeek. The provider architecture is modular, making it straightforward to add additional providers like Anthropic, Cohere, or custom endpoints.",
  },
  {
    question: "Why are you selling instead of building a company around it?",
    answer: "We believe VEX is most valuable as infrastructure within a larger platform play. We're focused on other projects at ProvnAI and want to see this technology deployed at scale. The right acquirer can leverage VEX as a competitive advantage in ways we couldn't as an independent project.",
  },
];

export function FAQ() {
  const [openIndex, setOpenIndex] = useState<number | null>(0);

  return (
    <section className="py-32 px-4">
      <div className="max-w-3xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="text-center mb-16 space-y-4"
        >
          <span className="text-primary text-sm font-medium uppercase tracking-wider">Questions</span>
          <h2 className="text-4xl md:text-5xl font-bold text-white">
            Frequently <span className="text-primary">Asked</span>
          </h2>
          <p className="text-zinc-400 max-w-xl mx-auto text-lg">
            Common questions about the VEX acquisition opportunity
          </p>
        </motion.div>

        <div className="space-y-4">
          {faqs.map((faq, i) => (
            <motion.div
              key={i}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ delay: i * 0.05 }}
              className="border border-zinc-800 rounded-xl overflow-hidden hover:border-zinc-700 transition-colors"
            >
              <button
                onClick={() => setOpenIndex(openIndex === i ? null : i)}
                className="w-full flex items-center justify-between p-5 text-left bg-zinc-900/50 hover:bg-zinc-900 transition-colors"
              >
                <span className="font-medium text-white pr-4">{faq.question}</span>
                <span className="shrink-0 p-1 rounded-lg bg-zinc-800">
                  {openIndex === i ? (
                    <Minus className="w-4 h-4 text-primary" />
                  ) : (
                    <Plus className="w-4 h-4 text-zinc-400" />
                  )}
                </span>
              </button>
              
              <AnimatePresence>
                {openIndex === i && (
                  <motion.div
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: "auto", opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    transition={{ duration: 0.2 }}
                    className="overflow-hidden"
                  >
                    <div className="p-5 pt-0 text-zinc-400 leading-relaxed">
                      {faq.answer}
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}
