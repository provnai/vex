"use client";

import { motion } from "framer-motion";
import { useState } from "react";
import { ArrowRight, CheckCircle, Loader2, Sparkles } from "lucide-react";

export function WaitlistForm() {
  const [email, setEmail] = useState("");
  const [status, setStatus] = useState<"idle" | "loading" | "success" | "error">("idle");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!email) return;
    
    setStatus("loading");
    
    // Simulate API call - replace with actual endpoint
    await new Promise((resolve) => setTimeout(resolve, 1000));
    
    // Store in localStorage for demo (replace with real backend)
    const waitlist = JSON.parse(localStorage.getItem("vex-waitlist") || "[]");
    waitlist.push({ email, timestamp: new Date().toISOString() });
    localStorage.setItem("vex-waitlist", JSON.stringify(waitlist));
    
    setStatus("success");
    setEmail("");
  };

  return (
    <section className="relative py-32 px-4">
      {/* Background gradient */}
      <div className="absolute inset-0 bg-linear-to-b from-transparent via-primary/5 to-transparent" />
      
      <motion.div
        initial={{ opacity: 0, y: 40 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        className="relative max-w-2xl mx-auto text-center space-y-8"
      >
        <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-primary/10 border border-primary/20 text-primary text-sm font-medium">
          <Sparkles className="w-4 h-4" />
          Early Access
        </div>

        <h2 className="text-4xl md:text-5xl font-bold text-white">
          Get <span className="text-primary">Priority Access</span>
        </h2>
        
        <p className="text-xl text-zinc-400 max-w-xl mx-auto">
          Join the waitlist for exclusive early access, enterprise licensing opportunities, and direct support from the core team.
        </p>

        {status === "success" ? (
          <motion.div
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            className="flex items-center justify-center gap-3 py-4 px-6 bg-green-500/10 border border-green-500/20 rounded-xl text-green-400"
          >
            <CheckCircle className="w-5 h-5" />
            <span className="font-medium">You&apos;re on the list! We&apos;ll be in touch soon.</span>
          </motion.div>
        ) : (
          <form onSubmit={handleSubmit} className="flex flex-col sm:flex-row gap-3 max-w-md mx-auto">
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="Enter your email"
              required
              className="flex-1 px-5 py-4 bg-zinc-900 border border-zinc-800 rounded-xl text-white placeholder:text-zinc-500 focus:outline-none focus:border-primary/50 focus:ring-2 focus:ring-primary/20 transition-all"
            />
            <button
              type="submit"
              disabled={status === "loading"}
              className="group px-6 py-4 bg-primary hover:bg-primary/90 text-white font-semibold rounded-xl transition-all flex items-center justify-center gap-2 disabled:opacity-50"
            >
              {status === "loading" ? (
                <Loader2 className="w-5 h-5 animate-spin" />
              ) : (
                <>
                  Join Waitlist
                  <ArrowRight className="w-4 h-4 group-hover:translate-x-1 transition-transform" />
                </>
              )}
            </button>
          </form>
        )}

        <p className="text-sm text-zinc-600">
          No spam. Unsubscribe anytime. We respect your privacy.
        </p>
      </motion.div>
    </section>
  );
}
