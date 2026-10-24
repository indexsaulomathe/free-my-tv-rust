# Site freemytv.io (`docs/`)

HTML, CSS e JS estáticos, sem etapa de build. `docs/index.html` (inglês) e `docs/pt/index.html` (português) são a
mesma página. O workflow `.github/workflows/pages.yml` publica no GitHub Pages a cada push na `main`. **No fork,
os workflows ficam desligados até você ativá-los.**

`docs/webos/apps.json` é o repositório do webOS Homebrew Channel, gerado por `tools/homebrew_repo.py`.

Por isso esta documentação fica em `documentacao/`, não em `docs/`: o que estiver em `docs/` vai para o site.
