# Documentação do Free My TV (fork)

Esta pasta explica, em português, o que cada parte do projeto faz, como testar e o andamento da conversão para Rust.
O código original fica onde está (`app/`, `web/`, `webos/`, `tizen/`, `tools/`), para o fork continuar
sincronizável com o upstream (`simbasjungles/free-my-tv`). Tudo o que é novo mora em pastas novas:

| Pasta | O que tem |
|---|---|
| `documentacao/` | Esta documentação |
| `tests/` | Testes automatizados do código original (Python e JavaScript) |
| `rust/` | A conversão para Rust, componente por componente |

## Mapa dos componentes

O projeto tem quatro camadas. Cada uma tem seu documento:

1. **[Ferramentas](ferramentas.md)**: scripts Python que montam e instalam os pacotes (`webos/`, `tizen/`, `tools/`).
2. **[App web (LG e Samsung)](app-web.md)**: a tela inicial em JavaScript ES5 (`web/`).
3. **[App Android / Google TV](app-android.md)**: a tela inicial nativa em Java (`app/`).
4. **[Site](site.md)**: freemytv.io (`docs/`), HTML estático.

```
config.properties ──► ferramentas (Python / Rust) ──► FreeMyTV-webos.ipk ──► TV LG (SSH :9922)
                                │                 └─► FreeMyTV-tizen.wgt ──► TV Samsung (SDB :26101)
                                │
        web/ (JS) ──────────────┘  (empacotado junto com ícones, fontes e traduções)

build.sh ──► app/ (Java) ──► FreeMyTV.apk ──► Google TV (adb :5555)
```

Os três apps (Android, LG e Samsung) usam os mesmos serviços externos, todos por HTTPS:
Open-Meteo (clima), TMDB (sugestões, com chave do usuário), Binance (cripto) e, opcional, Watchmode.

## Outros documentos

- **[Testes](testes.md)**: como rodar e o que cada teste cobre.
- **[Segurança](seguranca.md)**: análise de riscos do projeto.
- **[Conversão para Rust](rust.md)**: plano, o que já foi convertido e os benchmarks.
