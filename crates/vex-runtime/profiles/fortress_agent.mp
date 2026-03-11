module fortress.verify
exports { @intent }
imports { }
digest "0000000000000000000000000000000000000000000000000000000000000000"

;; The only permitted external side-effect
fn @log_safe(%msg: Str) -> i32 meta { } {
  bb0:
    ret const.i32 0
}

;; The restricted entry point for the agent intent
fn @intent() -> i32 meta { } {
  bb0:
    ;; AGENT_INTENT_SLOT
    ret const.i32 0
}
