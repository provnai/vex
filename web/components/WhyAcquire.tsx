"use client";

import { motion } from "framer-motion";
import { Quote } from "lucide-react";

const reasons = [
  {
    title: "Time-to-Market Advantage",
    description: "Skip 12-18 months of R&D. VEX is production-ready today with battle-tested code, comprehensive tests, and real-world benchmarks. Your team can focus on building differentiated features instead of infrastructure.",
  },
  {
    title: "Technical Moat",
    description: "Rust's memory safety eliminates entire classes of bugs that plague Python/JavaScript agent frameworks. Zero-copy architecture delivers 10-100x performance improvements over competing solutions.",
  },
  {
    title: "Compliance Ready",
    description: "Cryptographic audit trails and Merkle tree verification provide tamper-proof evidence of every agent action. Essential for regulated industries like finance, healthcare, and government.",
  },
  {
    title: "Scalable Architecture",
    description: "Modular crate design means you can adopt incrementally. Use just the core agent primitives, or deploy the full stack with persistence, queuing, and API layers.",
  },
  {
    title: "Clean IP Transfer",
    description: "No third-party code with restrictive licenses. No ongoing royalties. No revenue share. Full ownership with complete freedom to modify, rebrand, and commercialize.",
  },
  {
    title: "Knowledge Transfer Option",
    description: "Optional consultation with the development team to accelerate your integration. Architecture walkthroughs, code reviews, and best practices for your specific use case.",
  },
];

export function WhyAcquire() {
  return (
    <section id="why" className="py-32 px-4 relative overflow-hidden">
      {/* Background */}
      <div className="absolute inset-0 bg-linear-to-r from-primary/5 via-transparent to-purple-500/5" />
      
      <div className="max-w-6xl mx-auto relative">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="text-center mb-16 space-y-4"
        >
          <span className="text-primary text-sm font-medium uppercase tracking-wider">Strategic Value</span>
          <h2 className="text-4xl md:text-5xl font-bold text-white">
            Why <span className="text-primary">Acquire</span> VEX?
          </h2>
          <p className="text-zinc-400 max-w-2xl mx-auto text-lg">
            In the race to build production AI systems, infrastructure is the bottleneck. VEX removes it.
          </p>
        </motion.div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-16">
          {reasons.map((reason, i) => (
            <motion.div
              key={reason.title}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ delay: i * 0.1 }}
              className="p-6 bg-zinc-900/50 border border-zinc-800 rounded-2xl hover:border-zinc-700 transition-all"
            >
              <div className="flex items-start gap-4">
                <span className="shrink-0 w-8 h-8 bg-primary/10 rounded-lg flex items-center justify-center text-primary font-bold">
                  {i + 1}
                </span>
                <div>
                  <h3 className="text-lg font-semibold text-white mb-2">{reason.title}</h3>
                  <p className="text-zinc-400 text-sm leading-relaxed">{reason.description}</p>
                </div>
              </div>
            </motion.div>
          ))}
        </div>

        {/* Quote/Testimonial style box */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="relative p-8 md:p-12 bg-linear-to-br from-primary/10 to-purple-500/10 border border-primary/20 rounded-3xl"
        >
          <Quote className="absolute top-6 left-6 w-8 h-8 text-primary/30" />
          <div className="relative text-center max-w-3xl mx-auto">
            <p className="text-xl md:text-2xl text-white font-medium leading-relaxed mb-6">
              &ldquo;The AI infrastructure market is projected to reach $150B by 2028. 
              Companies that control the foundational layer will capture disproportionate value. 
              VEX provides that foundation.&rdquo;
            </p>
            <p className="text-zinc-500 text-sm">
              Production-ready • Fully documented • Clean IP • Immediate deployment
            </p>
          </div>
        </motion.div>
      </div>
    </section>
  );
}
