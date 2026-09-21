#!/usr/bin/env bash
# Creates the Innards products and prices in Stripe (idempotent via lookup_key)
# and prints the price ids to paste into wrangler.toml [vars].
#   STRIPE_SECRET_KEY=sk_live_… ./scripts/setup-stripe.sh
set -euo pipefail
: "${STRIPE_SECRET_KEY:?set STRIPE_SECRET_KEY}"
api() { curl -sS -u "$STRIPE_SECRET_KEY:" "https://api.stripe.com/v1/$1" "${@:2}"; }

price_by_lookup() { api "prices?lookup_keys[]=$1&limit=1" | python3 -c 'import sys,json; d=json.load(sys.stdin)["data"]; print(d[0]["id"] if d else "")'; }
product_by_name() { api "products/search" --data-urlencode "query=name:'$1'" | python3 -c 'import sys,json; d=json.load(sys.stdin)["data"]; print(d[0]["id"] if d else "")'; }
ensure_product() { local id; id=$(product_by_name "$1"); [ -n "$id" ] && { echo "$id"; return; }; api products -d "name=$1" -d "description=$2" -d "metadata[product]=innards" | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])'; }

P_SUP=$(ensure_product "Innards Supporter" "One-time unlock: upgrade advisor, narrated summaries, machine history.")
P_PRO=$(ensure_product "Innards Pro" "Yearly: where-to-buy links and multi-machine view.")
P_TEAM=$(ensure_product "Innards Team" "Per machine per month: fleet dashboard on Threadwise.")

SUP=$(price_by_lookup innards_supporter)
[ -z "$SUP" ] && SUP=$(api prices -d "product=$P_SUP" -d "currency=usd" -d "lookup_key=innards_supporter" \
  -d "custom_unit_amount[enabled]=true" -d "custom_unit_amount[minimum]=500" -d "custom_unit_amount[preset]=900" -d "custom_unit_amount[maximum]=10000" \
  -d "tax_behavior=exclusive" | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])')

PRO=$(price_by_lookup innards_pro)
[ -z "$PRO" ] && PRO=$(api prices -d "product=$P_PRO" -d "currency=usd" -d "unit_amount=2900" -d "recurring[interval]=year" -d "lookup_key=innards_pro" -d "tax_behavior=exclusive" | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])')

TEAM=$(price_by_lookup innards_team)
[ -z "$TEAM" ] && TEAM=$(api prices -d "product=$P_TEAM" -d "currency=usd" -d "unit_amount=400" -d "recurring[interval]=month" -d "lookup_key=innards_team" -d "tax_behavior=exclusive" | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])')

echo "PRICE_SUPPORTER = \"$SUP\""
echo "PRICE_PRO = \"$PRO\""
echo "PRICE_TEAM = \"$TEAM\""
