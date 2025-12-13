"use client";

import { motion } from "framer-motion";
import { 
  Package, 
  FileCode, 
  Users, 
  BookOpen, 
  Shield, 
  ArrowRight,
  CheckCircle,
  Clock,
  Briefcase
} from "lucide-react";
import Link from "next/link";

const included = [
  {
    icon: FileCode,
    title: "Complete Source Code",
    description: "15,000+ lines of production Rust code across 10 modular crates",
  },
  {
    icon: Package,
    title: "All IP & Assets",
    description: "Trademarks, documentation, benchmarks, and technical specifications",
  },
  {
    icon: BookOpen,
    title: "Technical Documentation",
    description: "Architecture docs, API references, and implementation guides",
  },
  {
    icon: Users,
    title: "Knowledge Transfer",
    description: "Optional consultation period with the development team",
  },
  {
    icon: Shield,
    title: "Clean IP History",
    description: "Full audit trail, no third-party dependencies with restrictive licenses",
  },
  {
    icon: Briefcase,
    title: "Commercial Rights",
    description: "Full exclusive ownership with no ongoing royalties or restrictions",
  },
];

const benefits = [
  "Skip 12+ months of R&D development time",
  "Battle-tested architecture with proven benchmarks",
  "Modular design ready for your specific use case",
  "Memory-safe Rust codebase eliminates entire bug classes",
  "Built-in cryptographic audit trail for compliance",
  "Multi-provider LLM support (OpenAI, Ollama, DeepSeek)",
];

export function Acquisition() {
  return (
    <section id="acquisition" className="py-32 px-4 relative overflow-hidden">
      {/* Background effect */}
      <div className="absolute inset-0 bg-linear-to-b from-transparent via-primary/5 to-transparent" />
      
      <div className="max-w-6xl mx-auto relative">
        {/* Header */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="text-center mb-20 space-y-6"
        >
          <motion.div
            initial={{ scale: 0.9 }}
            whileInView={{ scale: 1 }}
            viewport={{ once: true }}
            className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-primary/10 border border-primary/20 text-primary text-sm font-medium"
          >
            <span className="relative flex h-2 w-2">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary opacity-75"></span>
              <span className="relative inline-flex rounded-full h-2 w-2 bg-primary"></span>
            </span>
            Exclusive Opportunity
          </motion.div>
          
          <h2 className="text-4xl md:text-6xl font-bold text-white">
            Acquire <span className="text-primary">Complete Ownership</span>
          </h2>
          
          <p className="text-xl text-zinc-400 max-w-2xl mx-auto">
            A rare opportunity to own production-ready AI agent infrastructure. 
            No licensing, no revenue share — full IP transfer.
          </p>
        </motion.div>

        {/* What's included grid */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="mb-20"
        >
          <h3 className="text-2xl font-bold text-white mb-8 text-center">What&apos;s Included</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {included.map((item, i) => (
              <motion.div
                key={item.title}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ delay: i * 0.1 }}
                whileHover={{ scale: 1.02, borderColor: "rgba(59,130,246,0.5)" }}
                className="p-6 bg-zinc-900/50 border border-zinc-800 rounded-2xl transition-all"
              >
                <div className="w-12 h-12 bg-primary/10 rounded-xl flex items-center justify-center mb-4">
                  <item.icon className="w-6 h-6 text-primary" />
                </div>
                <h4 className="text-lg font-semibold text-white mb-2">{item.title}</h4>
                <p className="text-zinc-400">{item.description}</p>
              </motion.div>
            ))}
          </div>
        </motion.div>

        {/* Benefits section */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="mb-20 bg-linear-to-r from-zinc-900 to-zinc-900/50 border border-zinc-800 rounded-3xl p-10"
        >
          <h3 className="text-2xl font-bold text-white mb-8">Why Acquire VEX?</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {benefits.map((benefit, i) => (
              <motion.div
                key={i}
                initial={{ opacity: 0, x: -20 }}
                whileInView={{ opacity: 1, x: 0 }}
                viewport={{ once: true }}
                transition={{ delay: i * 0.1 }}
                className="flex items-start gap-3"
              >
                <CheckCircle className="w-5 h-5 text-green-500 mt-0.5 shrink-0" />
                <span className="text-zinc-300">{benefit}</span>
              </motion.div>
            ))}
          </div>
        </motion.div>

        {/* CTA Section */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="text-center space-y-8"
        >
          <div className="bg-linear-to-r from-primary/10 via-primary/5 to-primary/10 border border-primary/20 rounded-3xl p-10 md:p-16">
            <motion.div
              animate={{ scale: [1, 1.02, 1] }}
              transition={{ duration: 3, repeat: Infinity }}
            >
              <h3 className="text-3xl md:text-4xl font-bold text-white mb-4">
                Ready to Own the Future of AI Agents?
              </h3>
            </motion.div>
            <p className="text-zinc-400 mb-8 max-w-xl mx-auto">
              Schedule a confidential call to discuss acquisition details, view the codebase, and explore how VEX can accelerate your AI strategy.
            </p>
            
            <div className="flex flex-col sm:flex-row gap-4 justify-center items-center">
              <Link
                href="mailto:info@moreclients.be?subject=VEX%20Framework%20Acquisition%20Inquiry"
                className="group px-10 py-5 bg-primary hover:bg-primary/90 text-white font-semibold rounded-xl transition-all flex items-center gap-3 shadow-lg shadow-primary/25 text-lg"
              >
                <Shield className="w-5 h-5" />
                Request NDA & Details
                <ArrowRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
              </Link>
              <Link
                href="https://x.com/provnai"
                target="_blank"
                className="px-10 py-5 bg-zinc-900 border border-zinc-800 text-zinc-300 rounded-xl hover:bg-zinc-800 hover:border-zinc-700 transition-all font-medium flex items-center gap-2 text-lg"
              >
                Follow @provnai
              </Link>
            </div>

            <div className="mt-8 flex items-center justify-center gap-6 text-sm text-zinc-500">
              <span className="flex items-center gap-2">
                <Clock className="w-4 h-4" />
                Response within 24h
              </span>
              <span className="flex items-center gap-2">
                <Shield className="w-4 h-4" />
                NDA Protected
              </span>
            </div>
          </div>
        </motion.div>
      </div>
    </section>
  );
}
