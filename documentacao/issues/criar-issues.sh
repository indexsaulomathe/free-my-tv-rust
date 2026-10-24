#!/usr/bin/env bash
# Abre no GitHub uma issue para cada arquivo NN-*.md desta pasta, com os rótulos do cabeçalho.
# As de "status: feito" são abertas e fechadas logo em seguida, para o histórico ficar registrado.
#
#   documentacao/issues/criar-issues.sh                 # no repositório indexsaulomathe/free-my-tv-rust
#   documentacao/issues/criar-issues.sh dono/repositorio
#
# Precisa do GitHub CLI (gh) logado e das Issues ativadas no fork (Settings → General → Features → Issues).
# Rodar duas vezes cria issues duplicadas: ele avisa e para se já existir uma com o mesmo título.
set -euo pipefail
cd "$(dirname "$0")"
repo="${1:-indexsaulomathe/free-my-tv-rust}"

field() { sed -n "s/^$1: *//p" "$2" | head -1; }

for label in conversao-rust fase-1-ferramentas fase-2-logica fase-3-interface seguranca; do
  gh label create "$label" -R "$repo" --force >/dev/null
done

for f in [0-9][0-9]-*.md; do
  title="$(field title "$f" | sed 's/^"//; s/"$//')"
  labels="$(field labels "$f" | tr -d '[] ')"
  status="$(field status "$f")"
  if gh issue list -R "$repo" --state all --search "in:title \"$title\"" --json title --jq '.[].title' | grep -qxF "$title"; then
    echo "já existe: $title"
    continue
  fi
  body="$(awk 'BEGIN{n=0} /^---$/{n++; next} n>=2' "$f")"
  url="$(gh issue create -R "$repo" --title "$title" --label "$labels" --body "$body")"
  echo "$url  $title"
  if [ "$status" = feito ]; then
    gh issue close "$url" -R "$repo" --reason completed --comment "Feito na branch claude/testes-docs-rust (veja documentacao/alteracoes.md)." >/dev/null
  fi
done
