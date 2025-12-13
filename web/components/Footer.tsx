"use client";

import { Twitter, Mail, ExternalLink } from "lucide-react";
import Link from "next/link";
import { motion } from "framer-motion";

export function Footer() {
  return (
    <footer className="py-16 px-4 border-t border-zinc-800/50 bg-zinc-950/50">
      <div className="max-w-6xl mx-auto">
        <div className="flex flex-col md:flex-row justify-between items-center gap-8">
          {/* Brand */}
          <motion.div 
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className="flex flex-col items-center md:items-start gap-2"
          >
            <div className="text-3xl font-bold text-white">
              VEX<span className="text-primary">.</span>
            </div>
            <p className="text-sm text-zinc-500">
              A <Link href="https://www.provnai.com" target="_blank" className="text-primary hover:underline">ProvnAI</Link> Project
            </p>
          </motion.div>

          {/* Links */}
          <motion.div 
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ delay: 0.1 }}
            className="flex items-center gap-8 text-sm text-zinc-400"
          >
            <Link href="#details" className="hover:text-white transition-colors">
              Technical Details
            </Link>
            <Link href="#acquisition" className="hover:text-white transition-colors">
              Acquisition
            </Link>
            <Link href="https://www.provnai.com" target="_blank" className="hover:text-white transition-colors flex items-center gap-1">
              ProvnAI
              <ExternalLink className="w-3 h-3" />
            </Link>
          </motion.div>

          {/* Social */}
          <motion.div 
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ delay: 0.2 }}
            className="flex items-center gap-4"
          >
            <Link
              href="https://x.com/provnai"
              target="_blank"
              className="p-3 text-zinc-500 hover:text-white hover:bg-zinc-800 rounded-xl transition-all"
            >
              <Twitter className="w-5 h-5" />
            </Link>
            <Link
              href="mailto:info@moreclients.be"
              className="p-3 text-zinc-500 hover:text-white hover:bg-zinc-800 rounded-xl transition-all"
            >
              <Mail className="w-5 h-5" />
            </Link>
          </motion.div>
        </div>

        <motion.div 
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          transition={{ delay: 0.3 }}
          className="mt-12 pt-8 border-t border-zinc-800/50 text-center text-sm text-zinc-600"
        >
          © {new Date().getFullYear()} <Link href="https://www.provnai.com" target="_blank" className="hover:text-zinc-400">ProvnAI</Link>. All Rights Reserved. VEX Framework — Built with 🦀 Rust.
        </motion.div>
      </div>
    </footer>
  );
}
