---
title: "Converter validação das traduções (translate.py --check)"
labels: [fase-1-ferramentas, conversao-rust]
status: feito
---

**Origem:** tools/translate.py: extract, check, fits, valid_format, problems
**Destino:** `freemytv translate-check`

Extrair chaves do Java e do JS e validar todos os catálogos (chaves, placeholders, plurais CLDR, padrões de data). O `--fill` (API da Anthropic) fica para depois.

## Critérios de aceite
- [ ] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [ ] Testes unitários no crate
- [ ] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [ ] Benchmark em `tests/bench/bench.py`, quando aplicável
