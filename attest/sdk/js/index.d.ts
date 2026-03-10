export class AttestError extends Error {
  code: string;
  details: Record<string, unknown> | null;
  constructor(message: string, code?: string, details?: Record<string, unknown> | null);
}

export class AttestCLIError extends AttestError {
  exitCode: number;
  stderr: string;
  constructor(message: string, exitCode: number, stderr: string);
}

export class AttestNotFoundError extends AttestError {
  resourceType: string;
  id: string;
  constructor(resourceType: string, id: string);
}

export class AttestConfigurationError extends AttestError {
  constructor(message: string);
}

export interface Agent {
  id: string;
  name: string;
  type: string;
  publicKey: string;
  createdAt: string;
  revoked: boolean;
  revokedAt?: string;
  metadata?: Record<string, unknown>;
}

export interface Intent {
  id: string;
  goal: string;
  status: string;
  constraints: Record<string, unknown>;
  acceptanceCriteria: string[];
  agentId?: string;
  ticket?: string;
  createdAt: string;
  completedAt?: string;
}

export interface Attestation {
  id: string;
  agentId: string;
  agentName: string;
  actionType: string;
  actionTarget: string;
  timestamp: string;
  signature: string;
  intentId?: string;
  actionInput?: string;
  metadata?: Record<string, unknown>;
}

export interface ExecutionResult {
  id: string;
  command: string;
  status: string;
  workingDir: string;
  backupPath?: string;
  agentId?: string;
  intentId?: string;
  createdAt: string;
  rolledBackAt?: string;
}

export interface Policy {
  id: string;
  name: string;
  description: string;
  rules: PolicyRule[];
  enabled: boolean;
  createdAt: string;
}

export interface PolicyRule {
  action: string;
  target?: string;
  condition?: Record<string, unknown>;
  effect: 'allow' | 'deny';
}

export interface PolicyCheckResult {
  allowed: boolean;
  reason: string;
  matchingPolicies: Policy[];
}

export interface VerificationResult {
  valid: boolean;
  attestationId: string;
  agentId: string;
  timestamp: string;
  details?: Record<string, unknown>;
}

export interface VersionInfo {
  version: string;
  commit: string;
  date: string;
}

export interface StatusInfo {
  initialized: boolean;
  dataDir: string;
  agentsCount: number;
  attestationsCount: number;
  intentsCount: number;
}

export interface AttestClientOptions {
  cliPath?: string;
  dataDir?: string;
  verbose?: boolean;
}

export declare class AttestClient {
  constructor(options?: AttestClientOptions);
  version(): Promise<VersionInfo>;
  status(): Promise<StatusInfo>;
  init(dataDir?: string): Promise<{ message: string }>;
  agentCreate(name: string, options?: { type?: string; metadata?: Record<string, unknown> }): Promise<Agent>;
  agentList(options?: { includeRevoked?: boolean; format?: string }): Promise<Agent[]>;
  agentShow(agentId: string): Promise<Agent>;
  agentDelete(agentId: string): Promise<{ message: string }>;
  agentExport(agentId: string): Promise<Record<string, unknown>>;
  agentImport(filepath: string): Promise<Record<string, unknown>>;
  intentCreate(goal: string, options?: { constraints?: Record<string, unknown>; acceptanceCriteria?: string[]; agentId?: string; ticket?: string }): Promise<Intent>;
  intentList(options?: { status?: string; limit?: number; format?: string }): Promise<Intent[]>;
  intentShow(intentId: string): Promise<Intent>;
  intentLinkAction(intentId: string, actionId: string): Promise<{ message: string }>;
  attestAction(agentId: string, action: string, target: string, options?: { intentId?: string; input?: string; sessionId?: string }): Promise<Attestation>;
  attestList(options?: { agentId?: string; intentId?: string; limit?: number }): Promise<Attestation[]>;
  attestShow(attestationId: string): Promise<Attestation>;
  attestExport(attestationId: string, options?: { output?: string }): Promise<Record<string, unknown>>;
  attestImport(filepath: string): Promise<Record<string, unknown>>;
  verifyAttestation(attestationId: string): Promise<VerificationResult>;
  execRun(command: string, options?: { reversible?: boolean; agentId?: string; intentId?: string; backupType?: string; dryRun?: boolean; sessionId?: string }): Promise<ExecutionResult>;
  rollback(actionId?: string): Promise<ExecutionResult>;
  execHistory(options?: { pendingOnly?: boolean }): Promise<ExecutionResult[]>;
  policyCheck(action: string, target: string, options?: { agentId?: string; intentId?: string }): Promise<PolicyCheckResult>;
  policyList(): Promise<Policy[]>;
  policyAdd(filepath: string): Promise<Record<string, unknown>>;
  policyRemove(policyId: string): Promise<{ message: string }>;
}

export declare const createClient: (options?: AttestClientOptions) => AttestClient;

export { AttestError, AttestCLIError, AttestNotFoundError, AttestConfigurationError };
