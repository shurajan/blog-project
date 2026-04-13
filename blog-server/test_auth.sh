#!/usr/bin/env bash
# Smoke-тесты для /api/auth/register и /api/auth/login.
# Требует: curl, jq.

set -u  # не set -e: мы сами решаем, что считать провалом

BASE_URL="${BASE_URL:-http://localhost:8080}"
# Уникальный суффикс, чтобы повторные запуски не ломались о UserAlreadyExists.
SUFFIX="$(date +%s)"
USERNAME="alice_${SUFFIX}"
EMAIL="alice_${SUFFIX}@example.com"
PASSWORD="Secret123"

PASS=0
FAIL=0

# --- утилиты ------------------------------------------------------------------

c_red()   { printf '\033[31m%s\033[0m' "$1"; }
c_green() { printf '\033[32m%s\033[0m' "$1"; }
c_dim()   { printf '\033[2m%s\033[0m'  "$1"; }

# call METHOD PATH JSON_BODY  →  печатает "HTTP_CODE<TAB>BODY"
call() {
    local method="$1" path="$2" body="$3"
    curl -sS -X "$method" "${BASE_URL}${path}" \
        -H "Content-Type: application/json" \
        -d "$body" \
        -w $'\n%{http_code}'
}

# expect NAME EXPECTED_CODE METHOD PATH BODY
expect() {
    local name="$1" expected="$2" method="$3" path="$4" body="$5"
    local raw http_code response_body

    raw="$(call "$method" "$path" "$body")"
    http_code="$(printf '%s' "$raw" | tail -n1)"
    response_body="$(printf '%s' "$raw" | sed '$d')"

    printf '  %s %s → ' "$method" "$path"
    if [[ "$http_code" == "$expected" ]]; then
        c_green "OK"
        printf ' (%s)\n' "$http_code"
        PASS=$((PASS + 1))
    else
        c_red "FAIL"
        printf ' (expected %s, got %s)\n' "$expected" "$http_code"
        printf '    body: '
        c_dim "$response_body"
        printf '\n'
        FAIL=$((FAIL + 1))
    fi

    # возвращаем тело в глобальную переменную для дальнейших шагов
    LAST_BODY="$response_body"
}

section() {
    printf '\n\033[1m%s\033[0m\n' "$1"
}

# --- проверка окружения -------------------------------------------------------

command -v jq   >/dev/null || { echo "jq is required";   exit 2; }
command -v curl >/dev/null || { echo "curl is required"; exit 2; }

echo "Base URL: $BASE_URL"
echo "Test user: $USERNAME / $EMAIL"

# --- тесты --------------------------------------------------------------------

section "1. register — happy path (expect 201)"
expect "register" 201 POST /api/auth/register \
    "$(jq -n --arg u "$USERNAME" --arg e "$EMAIL" --arg p "$PASSWORD" \
        '{username:$u, email:$e, password:$p}')"

REGISTER_TOKEN="$(printf '%s' "$LAST_BODY" | jq -r '.token // empty')"
if [[ -n "$REGISTER_TOKEN" ]]; then
    printf '    token: '
    c_dim "${REGISTER_TOKEN:0:32}..."
    printf '\n'
else
    c_red "    WARN: no .token field in response"
    printf '\n'
fi

section "2. register — duplicate (expect 409)"
expect "register duplicate" 409 POST /api/auth/register \
    "$(jq -n --arg u "$USERNAME" --arg e "$EMAIL" --arg p "$PASSWORD" \
        '{username:$u, email:$e, password:$p}')"

section "3. login — happy path (expect 200)"
expect "login" 200 POST /api/auth/login \
    "$(jq -n --arg u "$USERNAME" --arg p "$PASSWORD" \
        '{username:$u, password:$p}')"

LOGIN_TOKEN="$(printf '%s' "$LAST_BODY" | jq -r '.token // empty')"
[[ -n "$LOGIN_TOKEN" ]] && {
    printf '    token: '
    c_dim "${LOGIN_TOKEN:0:32}..."
    printf '\n'
}

section "4. login — wrong password (expect 401)"
expect "login wrong password" 401 POST /api/auth/login \
    "$(jq -n --arg u "$USERNAME" \
        '{username:$u, password:"totally_wrong"}')"

section "5. login — unknown user (expect 401, NOT 404)"
expect "login unknown user" 401 POST /api/auth/login \
    '{"username":"ghost_user_does_not_exist","password":"whatever"}'

section "6. register — missing field (expect 400)"
expect "register missing password" 400 POST /api/auth/register \
    '{"username":"bob","email":"bob@example.com"}'

section "7. register — malformed JSON (expect 400)"
expect "register malformed json" 400 POST /api/auth/register \
    '{"username":"bob"'

# --- итог ---------------------------------------------------------------------

printf '\n'
printf 'Results: '
c_green "$PASS passed"
printf ', '
if [[ "$FAIL" -gt 0 ]]; then
    c_red "$FAIL failed"
else
    printf '0 failed'
fi
printf '\n'

exit $(( FAIL > 0 ? 1 : 0 ))