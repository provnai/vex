"use client";

import { motion } from "framer-motion";
import { 
  TrendingUp, 
  FileSearch, 
  Bot, 
  Shield, 
  Workflow,
  MessageSquare
} from "lucide-react";

const useCases = [
  {
    icon: TrendingUp,
    title: "Financial Analysis Agents",
    description: "Deploy agents that analyze market data, generate reports, and make recommendations with full audit trails for compliance.",
    metrics: "Real-time processing • SEC compliant • Audit ready",
  },
  {
    icon: FileSearch,
    title: "Research & Due Diligence",
    description: "Autonomous research agents that gather, synthesize, and verify information across multiple sources with citation tracking.",
    metrics: "Multi-source • Fact verification • Citation graphs",
  },
  {
    icon: Bot,
    title: "Customer Service Automation",
    description: "Intelligent agents that handle customer inquiries with context awareness and seamless escalation to human operators.",
    metrics: "Context-aware • Escalation logic • Sentiment analysis",
  },
  {
    icon: Shield,
    title: "Security & Compliance",
    description: "Agents that monitor systems, detect anomalies, and ensure regulatory compliance with cryptographic proof of actions.",
    metrics: "Real-time monitoring • Tamper-proof logs • Alerts",
  },
  {
    icon: Workflow,
    title: "Workflow Orchestration",
    description: "Coordinate complex multi-step processes across systems with intelligent error handling and recovery.",
    metrics: "Multi-agent • Error recovery • State persistence",
  },
  {
    icon: MessageSquare,
    title: "Content Generation Pipelines",
    description: "Production content systems with review workflows, version control, and quality assurance built-in.",
    metrics: "Quality gates • Versioning • Review workflows",
  },
];

export function UseCases() {
  return (
    <section className="py-32 px-4 relative">
      <div className="absolute inset-0 bg-linear-to-b from-zinc-900/50 via-transparent to-zinc-900/50" />
      
      <div className="max-w-6xl mx-auto relative">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="text-center mb-16 space-y-4"
        >
          <span className="text-primary text-sm font-medium uppercase tracking-wider">Applications</span>
          <h2 className="text-4xl md:text-5xl font-bold text-white">
            Built for <span className="text-primary">Real-World</span> Use Cases
          </h2>
          <p className="text-zinc-400 max-w-2xl mx-auto text-lg">
            VEX powers production AI systems across industries where reliability, security, and auditability are non-negotiable.
          </p>
        </motion.div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {useCases.map((useCase, i) => (
            <motion.div
              key={useCase.title}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ delay: i * 0.1 }}
              whileHover={{ y: -5 }}
              className="group p-6 bg-zinc-900/80 border border-zinc-800 rounded-2xl hover:border-primary/30 transition-all duration-300"
            >
              <div className="flex items-start gap-4">
                <div className="p-3 bg-primary/10 rounded-xl group-hover:bg-primary/20 transition-colors">
                  <useCase.icon className="w-6 h-6 text-primary" />
                </div>
                <div className="flex-1">
                  <h3 className="text-lg font-semibold text-white mb-2">{useCase.title}</h3>
                  <p className="text-zinc-400 text-sm leading-relaxed mb-3">{useCase.description}</p>
                  <p className="text-xs text-zinc-600 font-mono">{useCase.metrics}</p>
                </div>
              </div>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}
