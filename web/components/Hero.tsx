"use client";

import { motion } from "framer-motion";
import { ArrowRight, Shield, Sparkles } from "lucide-react";
import Link from "next/link";

export function Hero() {
  return (
    <section className="relative flex flex-col items-center justify-center min-h-[calc(100vh-4rem)] text-center px-4 overflow-hidden">
      {/* Animated background gradient orbs */}
      <div className="absolute inset-0 overflow-hidden">
        <motion.div 
          animate={{ 
            scale: [1, 1.2, 1],
            opacity: [0.3, 0.5, 0.3]
          }}
          transition={{ duration: 8, repeat: Infinity, ease: "easeInOut" }}
          className="absolute -top-40 -right-40 w-125 h-125 bg-primary/30 rounded-full blur-[120px]" 
        />
        <motion.div 
          animate={{ 
            scale: [1, 1.3, 1],
            opacity: [0.2, 0.4, 0.2]
          }}
          transition={{ duration: 10, repeat: Infinity, ease: "easeInOut", delay: 1 }}
          className="absolute -bottom-40 -left-40 w-125 h-125 bg-purple-500/20 rounded-full blur-[120px]" 
        />
        <motion.div 
          animate={{ 
            scale: [1, 1.1, 1],
          }}
          transition={{ duration: 6, repeat: Infinity, ease: "easeInOut", delay: 2 }}
          className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-150 h-150 bg-blue-500/10 rounded-full blur-[150px]" 
        />
      </div>

      {/* Grid background */}
      <div className="absolute inset-0 bg-grid z-0" />
      <div className="absolute inset-0 bg-linear-to-t from-background via-transparent to-background/80 z-0" />

      <div className="relative z-10 max-w-5xl mx-auto space-y-8">
        
        {/* Exclusive Badge */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
          className="flex items-center justify-center gap-3"
        >
          <motion.div 
            animate={{ boxShadow: ["0 0 20px rgba(59,130,246,0.3)", "0 0 40px rgba(59,130,246,0.5)", "0 0 20px rgba(59,130,246,0.3)"] }}
            transition={{ duration: 2, repeat: Infinity }}
            className="inline-flex items-center gap-2 px-5 py-2.5 rounded-full bg-linear-to-r from-primary/20 to-purple-500/20 border border-primary/30 text-white text-sm font-medium backdrop-blur-sm"
          >
            <Sparkles className="w-4 h-4 text-primary" />
            <span className="text-primary font-semibold">Exclusive Acquisition Opportunity</span>
          </motion.div>
        </motion.div>

        {/* Main Headline with gradient */}
        <motion.h1 
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.1 }}
          className="text-6xl md:text-8xl font-bold tracking-tighter"
        >
          <span className="bg-clip-text text-transparent bg-linear-to-b from-white via-white to-zinc-500">
            Own The Future
          </span>
          <br />
          <span className="bg-clip-text text-transparent bg-linear-to-r from-primary via-blue-400 to-purple-500">
            of AI Agents
          </span>
        </motion.h1>

        {/* Subheadline */}
        <motion.p 
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.2 }}
          className="text-xl md:text-2xl text-zinc-400 max-w-3xl mx-auto leading-relaxed"
        >
          <span className="text-white font-semibold">VEX Framework</span> — A complete, production-ready 
          <br className="hidden md:block" />
          Rust-native agentic infrastructure available for <span className="text-primary font-semibold">full acquisition</span>.
        </motion.p>

        {/* Key metrics */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.3 }}
          className="flex flex-wrap justify-center gap-6 text-sm"
        >
          {[
            { label: "550k+ ops/sec", icon: "⚡" },
            { label: "100% Rust", icon: "🦀" },
            { label: "Zero-Copy Architecture", icon: "🔒" },
            { label: "Production Ready", icon: "✓" },
          ].map((item, i) => (
            <motion.span
              key={item.label}
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
              transition={{ delay: 0.4 + i * 0.1 }}
              className="px-4 py-2 bg-zinc-900/80 border border-zinc-800 rounded-full text-zinc-300 backdrop-blur-sm"
            >
              {item.icon} {item.label}
            </motion.span>
          ))}
        </motion.div>

        {/* CTA Buttons */}
        <motion.div 
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.5 }}
          className="flex flex-col sm:flex-row gap-4 justify-center items-center pt-6"
        >
          <Link 
            href="#acquisition"
            className="group relative px-10 py-5 bg-primary hover:bg-primary/90 text-white font-semibold rounded-xl transition-all flex items-center gap-2 shadow-lg shadow-primary/25 text-lg"
          >
            <motion.span 
              className="absolute inset-0 rounded-xl bg-linear-to-r from-primary to-blue-500"
              animate={{ opacity: [0, 0.5, 0] }}
              transition={{ duration: 2, repeat: Infinity }}
            />
            <span className="relative flex items-center gap-2">
              <Shield className="w-5 h-5" />
              Inquire for Acquisition
              <ArrowRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
            </span>
          </Link>
          <Link 
            href="#details"
            className="px-10 py-5 bg-zinc-900 border border-zinc-800 text-zinc-300 rounded-xl hover:bg-zinc-800 hover:border-zinc-700 transition-all font-medium flex items-center gap-2 text-lg"
          >
            View Technical Details
          </Link>
        </motion.div>

        {/* Trust signal */}
        <motion.p 
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 1, delay: 0.7 }}
          className="pt-8 text-zinc-600 text-sm flex items-center justify-center gap-2"
        >
          <Shield className="w-4 h-4" />
          Protected IP • NDA Available • Serious Inquiries Only
        </motion.p>

      </div>

      {/* Scroll indicator */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 1 }}
        className="absolute bottom-8 left-1/2 -translate-x-1/2"
      >
        <div className="w-6 h-10 rounded-full border-2 border-zinc-700 flex items-start justify-center p-2">
          <motion.div
            animate={{ y: [0, 8, 0] }}
            transition={{ duration: 1.5, repeat: Infinity }}
            className="w-1.5 h-1.5 rounded-full bg-zinc-500"
          />
        </div>
      </motion.div>
    </section>
  );
}
