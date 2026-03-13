module standard
exports { @log_safe }
imports { }
digest "0000000000000000000000000000000000000000000000000000000000000000"

fn @log_safe(%msg: Str) -> unit meta { } {
  bb0:
    ret const.unit unit
}
