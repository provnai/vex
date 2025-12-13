"use client";

import { motion, useScroll, useTransform } from "framer-motion";
import { useState } from "react";
import Link from "next/link";
import { Menu, X, ArrowRight } from "lucide-react";

const navLinks = [
  { href: "#details", label: "Features" },
  { href: "#architecture", label: "Architecture" },
  { href: "#why", label: "Why VEX" },
  { href: "#acquisition", label: "Acquire" },
];

export function Navbar() {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const { scrollY } = useScroll();
  
  const backgroundColor = useTransform(
    scrollY,
    [0, 100],
    ["rgba(10, 10, 10, 0)", "rgba(10, 10, 10, 0.95)"]
  );
  
  const borderColor = useTransform(
    scrollY,
    [0, 100],
    ["rgba(39, 39, 42, 0)", "rgba(39, 39, 42, 1)"]
  );

  return (
    <>
      <motion.header
        style={{ backgroundColor, borderColor }}
        className="fixed top-0 left-0 right-0 z-50 border-b backdrop-blur-md"
      >
        <nav className="max-w-6xl mx-auto px-4 py-4 flex items-center justify-between">
          {/* Logo */}
          <Link href="/" className="text-2xl font-bold text-white">
            VEX<span className="text-primary">.</span>
          </Link>

          {/* Desktop Nav */}
          <div className="hidden md:flex items-center gap-8">
            {navLinks.map((link) => (
              <Link
                key={link.href}
                href={link.href}
                className="text-sm text-zinc-400 hover:text-white transition-colors"
              >
                {link.label}
              </Link>
            ))}
            <Link
              href="#acquisition"
              className="px-5 py-2.5 bg-primary hover:bg-primary/90 text-white text-sm font-medium rounded-lg transition-all flex items-center gap-2"
            >
              Inquire Now
              <ArrowRight className="w-4 h-4" />
            </Link>
          </div>

          {/* Mobile Menu Button */}
          <button
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            className="md:hidden p-2 text-zinc-400 hover:text-white"
          >
            {mobileMenuOpen ? <X className="w-6 h-6" /> : <Menu className="w-6 h-6" />}
          </button>
        </nav>

        {/* Mobile Menu */}
        {mobileMenuOpen && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            className="md:hidden bg-zinc-900/95 backdrop-blur-md border-t border-zinc-800"
          >
            <div className="px-4 py-6 space-y-4">
              {navLinks.map((link) => (
                <Link
                  key={link.href}
                  href={link.href}
                  onClick={() => setMobileMenuOpen(false)}
                  className="block text-zinc-400 hover:text-white transition-colors py-2"
                >
                  {link.label}
                </Link>
              ))}
              <Link
                href="#acquisition"
                onClick={() => setMobileMenuOpen(false)}
                className="block w-full px-5 py-3 bg-primary hover:bg-primary/90 text-white text-center font-medium rounded-lg transition-all"
              >
                Inquire Now
              </Link>
            </div>
          </motion.div>
        )}
      </motion.header>
    </>
  );
}
