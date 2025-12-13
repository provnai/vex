"use client";

import { motion, useMotionValue, useTransform, animate } from "framer-motion";
import { useEffect, useState } from "react";

interface CounterProps {
  value: number;
  suffix?: string;
  prefix?: string;
}

function Counter({ value, suffix = "", prefix = "" }: CounterProps) {
  const count = useMotionValue(0);
  const rounded = useTransform(count, (latest) => {
    if (value >= 1000) {
      return `${prefix}${Math.round(latest / 1000)}k${suffix}`;
    }
    return `${prefix}${Math.round(latest)}${suffix}`;
  });
  const [displayValue, setDisplayValue] = useState(`${prefix}0${suffix}`);

  useEffect(() => {
    const controls = animate(count, value, { duration: 2.5, ease: "easeOut" });
    const unsubscribe = rounded.on("change", (v) => setDisplayValue(v));
    return () => {
      controls.stop();
      unsubscribe();
    };
  }, [count, value, rounded]);

  return <span>{displayValue}</span>;
}

const stats = [
  { label: "Operations / Second", value: 550000, suffix: "+", description: "Benchmark tested" },
  { label: "Lines of Rust", value: 15000, suffix: "+", description: "Production code" },
  { label: "Memory Safety", value: 100, suffix: "%", description: "Zero unsafe blocks" },
  { label: "Test Coverage", value: 94, suffix: "%", description: "Comprehensive tests" },
];

export function Stats() {
  return (
    <section className="py-24 px-4 relative overflow-hidden">
      {/* Background gradient */}
      <div className="absolute inset-0 bg-linear-to-r from-primary/5 via-transparent to-purple-500/5" />
      
      <div className="max-w-6xl mx-auto relative">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="text-center mb-12"
        >
          <h2 className="text-2xl font-bold text-white mb-2">By The Numbers</h2>
          <p className="text-zinc-500">Real metrics from real benchmarks</p>
        </motion.div>

        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          className="grid grid-cols-2 md:grid-cols-4 gap-6 md:gap-8"
        >
          {stats.map((stat, i) => (
            <motion.div
              key={stat.label}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ delay: i * 0.1 }}
              whileHover={{ scale: 1.05 }}
              className="relative p-6 bg-zinc-900/50 border border-zinc-800 rounded-2xl text-center hover:border-zinc-700 transition-all"
            >
              <motion.div 
                className="text-4xl md:text-5xl font-bold text-white font-mono mb-2"
                initial={{ scale: 0.5 }}
                whileInView={{ scale: 1 }}
                viewport={{ once: true }}
                transition={{ delay: i * 0.1 + 0.2, type: "spring" }}
              >
                <Counter value={stat.value} suffix={stat.suffix} />
              </motion.div>
              <div className="text-sm text-zinc-400 font-medium mb-1">
                {stat.label}
              </div>
              <div className="text-xs text-zinc-600">
                {stat.description}
              </div>
            </motion.div>
          ))}
        </motion.div>
      </div>
    </section>
  );
}
