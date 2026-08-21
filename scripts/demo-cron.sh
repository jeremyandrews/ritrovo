#!/usr/bin/env sh
#
# The demo's cron poker.
#
# The Trovato kernel has no internal scheduler: tap_cron, the plugin-queue drain
# and the Pagefind index rebuild all happen inside a POST to /cron/<CRON_KEY> and
# nowhere else. On a real site that POST comes from system cron. Here it comes
# from this loop, which is the whole reason the demo reaches a populated site
# without anyone typing anything.
#
# Two phases, because the demo has two different cadences:
#
#   DRAIN    Right after the kernel starts, ritrovo_importer's tap_install is
#            filling the ritrovo_import queue with a batch per confs.tech
#            topic/year file, 2015 to now. One cron call drains at most 100 items
#            (the kernel's per-plugin fairness cap), so one call is nowhere near
#            enough. Poke fast until the queue has been empty for several
#            consecutive cycles — empty once is not drained, because tap_install
#            is still pushing.
#
#   STEADY   Then settle into a slow poke, so the site keeps behaving like a
#            cron'd site: the daily import cycle, the Pagefind rebuild, the
#            cleanup tasks.
#
# The kernel reports what it did, so the drain is observed rather than assumed:
# the response carries a "tap_queue_worker: N" task line, and N is the number of
# jobs that reached a terminal state this cycle.
#
# Environment (defaulted by docker-compose.demo.yml):
#   TROVATO_URL      base URL of the kernel        (http://trovato:3000)
#   CRON_KEY         the shared secret in the path
#   DRAIN_INTERVAL   seconds between drain pokes   (3)
#   STEADY_INTERVAL  seconds between steady pokes  (60)
#   QUIET_CYCLES     consecutive empty cycles that mean "drained" (10)
#   DRAIN_GRACE      seconds to keep waiting for the first job    (240)

set -eu

TROVATO_URL="${TROVATO_URL:-http://trovato:3000}"
CRON_KEY="${CRON_KEY:?CRON_KEY is required}"
DRAIN_INTERVAL="${DRAIN_INTERVAL:-3}"
STEADY_INTERVAL="${STEADY_INTERVAL:-60}"
QUIET_CYCLES="${QUIET_CYCLES:-10}"
DRAIN_GRACE="${DRAIN_GRACE:-240}"

say() { printf '[cron] %s\n' "$*"; }

# Poke cron once and echo the number of queue jobs that reached a terminal state.
#
# A cycle that ran no worker at all has no tap_queue_worker line, which is the
# same thing as zero. A "skipped" response means another instance holds the Redis
# cron lock; also zero, and the next poke picks it up.
poke() {
    body="$(curl -fsS -X POST "$TROVATO_URL/cron/$CRON_KEY" 2>/dev/null || true)"
    if [ -z "$body" ]; then
        echo "-1"
        return 0
    fi
    drained="$(printf '%s' "$body" | grep -o 'tap_queue_worker: [0-9]*' | grep -o '[0-9]*$' | head -n 1)"
    echo "${drained:-0}"
}

say "waiting for the kernel at $TROVATO_URL"
until curl -fsS "$TROVATO_URL/health" >/dev/null 2>&1; do
    sleep 2
done

say "draining the import queue (poking every ${DRAIN_INTERVAL}s)"
quiet=0
total=0
elapsed=0
while [ "$quiet" -lt "$QUIET_CYCLES" ]; do
    drained="$(poke)"
    if [ "$drained" = "-1" ]; then
        say "cron unreachable, retrying"
        quiet=0
    elif [ "$drained" -gt 0 ]; then
        total=$((total + drained))
        quiet=0
        say "drained $drained job(s), $total so far"
    elif [ "$total" -eq 0 ] && [ "$elapsed" -lt "$DRAIN_GRACE" ]; then
        # Nothing has arrived yet. tap_install runs in a background task at boot
        # and its first push waits on an HTTP round trip, so an empty queue this
        # early means "not started", not "finished" — and calling it finished
        # would have the demo report a drained queue before the import began.
        # After DRAIN_GRACE, stop making excuses for it and let the zeros count,
        # so an unreachable confs.tech ends the wait instead of hanging it.
        say "waiting for the first batch (${elapsed}s)"
    else
        quiet=$((quiet + 1))
        say "queue empty ($quiet/$QUIET_CYCLES consecutive)"
    fi
    sleep "$DRAIN_INTERVAL"
    elapsed=$((elapsed + DRAIN_INTERVAL))
done

say "import queue drained: $total job(s) processed"
say "steady state, poking every ${STEADY_INTERVAL}s"
while true; do
    poke >/dev/null
    sleep "$STEADY_INTERVAL"
done
