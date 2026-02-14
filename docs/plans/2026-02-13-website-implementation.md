# Website Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the tesseras.net project website as a Zola static site with warm monochrome styling, English + Brazilian Portuguese, no JavaScript.

**Architecture:** Zola static site generator with Tera templates. Single `base.html` layout with `<header>`, `<nav>`, `<main>`, `<footer>`. Content pages as Markdown with TOML front matter. Multilingual via Zola's built-in `languages` config with `.pt-br.md` file suffixes.

**Tech Stack:** Zola (static site generator), plain CSS (no Sass/preprocessor), semantic HTML5, Atom feeds (Zola built-in).

**Design doc:** `docs/plans/2026-02-13-website-design.md`

---

### Task 1: Scaffold Zola project and config

**Files:**
- Create: `website/config.toml`
- Create: `website/content/.gitkeep` (temporary, removed in later tasks)
- Create: `website/templates/.gitkeep` (temporary, removed in later tasks)
- Create: `website/static/.gitkeep` (temporary, removed in later tasks)

**Step 1: Initialize directory structure**

```bash
mkdir -p website/{content/news,templates,static}
```

**Step 2: Write config.toml**

Create `website/config.toml`:

```toml
base_url = "https://tesseras.net"
title = "Tesseras"
description = "P2P network for preserving human memories across millennia"
default_language = "en"
compile_sass = false
build_search_index = false
generate_feeds = true
feed_filenames = ["atom.xml"]

[markdown]
highlight_code = false

[languages.pt-br]
title = "Tesseras"
description = "Rede P2P para preservar memórias humanas através dos milênios"
generate_feeds = true
feed_filenames = ["atom.xml"]

[languages.pt-br.translations]

[translations]

[extra]
author = "Tesseras Project"
```

**Step 3: Verify Zola can load the config**

Run: `cd website && zola check`
Expected: Should complete without errors (may warn about missing templates, that's fine).

**Step 4: Commit**

```bash
git add website/
git commit -m "feat(website): scaffold Zola project with i18n config"
```

---

### Task 2: Create base template and CSS

**Files:**
- Create: `website/templates/base.html`
- Create: `website/static/style.css`

**Step 1: Write base.html**

Create `website/templates/base.html`:

```html
<!DOCTYPE html>
<html lang="{{ lang }}">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{% block title %}{{ config.title }}{% endblock %}</title>
    <meta name="description" content="{% block description %}{{ config.description }}{% endblock %}">
    <link rel="stylesheet" href="{{ get_url(path='style.css', cachebust=true) }}">
    {% if config.generate_feeds %}
        {% for feed in config.feed_filenames %}
            <link rel="alternate" type="application/atom+xml" title="{{ config.title }}" href="{{ get_url(path=feed) }}">
        {% endfor %}
    {% endif %}
    {% block head %}{% endblock %}
</head>
<body>
    <header>
        <h1><a href="{{ config.base_url }}/{% if lang != config.default_language %}{{ lang }}/{% endif %}">Tesseras</a></h1>
        <nav>
            {% if lang == "pt-br" %}
                <a href="{{ get_url(path='@/pages/about.pt-br.md') }}">Sobre</a>
                <a href="{{ get_url(path='@/news/_index.pt-br.md') }}">Notícias</a>
                <a href="{{ get_url(path='@/pages/releases.pt-br.md') }}">Lançamentos</a>
                <a href="{{ get_url(path='@/pages/faq.pt-br.md') }}">FAQ</a>
                <a href="{{ get_url(path='@/pages/subscriptions.pt-br.md') }}">Inscrições</a>
                <a href="{{ get_url(path='@/pages/contact.pt-br.md') }}">Contato</a>
            {% else %}
                <a href="{{ get_url(path='@/pages/about.md') }}">About</a>
                <a href="{{ get_url(path='@/news/_index.md') }}">News</a>
                <a href="{{ get_url(path='@/pages/releases.md') }}">Releases</a>
                <a href="{{ get_url(path='@/pages/faq.md') }}">FAQ</a>
                <a href="{{ get_url(path='@/pages/subscriptions.md') }}">Subscriptions</a>
                <a href="{{ get_url(path='@/pages/contact.md') }}">Contact</a>
            {% endif %}
        </nav>
        <nav class="lang-switch">
            {% if lang == "pt-br" %}
                <a href="{{ current_url | replace(from='/pt-br/', to='/') }}">English</a> | <strong>Português</strong>
            {% else %}
                <strong>English</strong> | <a href="/pt-br{{ current_path }}">Português</a>
            {% endif %}
        </nav>
    </header>

    <main>
        {% block content %}{% endblock %}
    </main>

    <footer>
        <p>&copy; {{ now() | date(format="%Y") }} Tesseras Project. <a href="/atom.xml">News Feed</a> · <a href="https://git.sr.ht/~ijanc/tesseras">Source</a></p>
    </footer>
</body>
</html>
```

**Note:** The nav links use `get_url(path='@/...')` which is Zola's internal linking. The exact paths depend on the content structure — we use a `pages/` section for standalone pages and `news/` for blog posts. The language switcher is simplified here; it will be refined during integration testing in Task 8.

**Step 2: Write style.css**

Create `website/static/style.css`:

```css
*,
*::before,
*::after {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen,
        Ubuntu, Cantarell, "Fira Sans", "Droid Sans", "Helvetica Neue", Arial,
        Noto Sans, sans-serif;
    font-size: 1rem;
    line-height: 1.6;
    color: #2c2c2c;
    background-color: #faf8f5;
    max-width: 42em;
    margin: 0 auto;
    padding: 2em 1em;
}

a {
    color: #4a4a4a;
    text-decoration: underline;
}

a:hover {
    color: #1a1a1a;
}

header {
    margin-bottom: 3em;
}

header h1 {
    font-size: 1.8rem;
    font-weight: 700;
    margin-bottom: 0.5em;
}

header h1 a {
    text-decoration: none;
    color: #2c2c2c;
}

header h1 a:hover {
    color: #1a1a1a;
}

nav {
    margin-bottom: 0.3em;
}

nav a {
    margin-right: 0.3em;
}

nav a:not(:last-child)::after {
    content: " |";
    color: #d4cfc8;
    text-decoration: none;
    margin-left: 0.3em;
}

nav.lang-switch {
    font-size: 0.85rem;
    margin-top: 0.3em;
}

nav.lang-switch a::after {
    content: "";
}

main {
    margin-bottom: 3em;
}

h2 {
    font-size: 1.3rem;
    font-weight: 600;
    margin-top: 1.5em;
    margin-bottom: 0.5em;
}

h3 {
    font-size: 1.1rem;
    font-weight: 600;
    margin-top: 1.2em;
    margin-bottom: 0.4em;
}

p {
    margin-bottom: 1em;
}

ul, ol {
    margin-bottom: 1em;
    padding-left: 1.5em;
}

li {
    margin-bottom: 0.3em;
}

code {
    font-family: "Fira Code", "Source Code Pro", "Cascadia Code", Consolas,
        "Liberation Mono", Menlo, monospace;
    background-color: #f0ece6;
    padding: 0.1em 0.3em;
    border-radius: 2px;
    font-size: 0.9em;
}

pre {
    background-color: #f0ece6;
    padding: 1em;
    overflow-x: auto;
    margin-bottom: 1em;
    border-radius: 2px;
}

pre code {
    padding: 0;
    background: none;
}

table {
    width: 100%;
    border-collapse: collapse;
    margin-bottom: 1em;
}

th, td {
    text-align: left;
    padding: 0.4em 0.6em;
    border-bottom: 1px solid #d4cfc8;
}

th {
    font-weight: 600;
    border-bottom: 2px solid #d4cfc8;
}

hr {
    border: none;
    border-top: 1px solid #d4cfc8;
    margin: 2em 0;
}

footer {
    border-top: 1px solid #d4cfc8;
    padding-top: 1em;
    font-size: 0.85rem;
    color: #6a6a6a;
}

.news-list {
    list-style: none;
    padding-left: 0;
}

.news-list li {
    margin-bottom: 1em;
    padding-bottom: 1em;
    border-bottom: 1px solid #d4cfc8;
}

.news-list li:last-child {
    border-bottom: none;
}

.news-date {
    font-size: 0.85rem;
    color: #6a6a6a;
}
```

**Step 3: Commit**

```bash
git add website/templates/base.html website/static/style.css
git commit -m "feat(website): add base template and CSS styling"
```

---

### Task 3: Create page templates

**Files:**
- Create: `website/templates/index.html`
- Create: `website/templates/page.html`
- Create: `website/templates/section.html`

**Step 1: Write index.html (homepage)**

Create `website/templates/index.html`:

```html
{% extends "base.html" %}

{% block title %}Tesseras — Preserve Your Memories Across Millennia{% endblock %}

{% block content %}
{{ section.content | safe }}
{% endblock %}
```

**Step 2: Write page.html (generic pages: FAQ, Contact, Releases, Subscriptions)**

Create `website/templates/page.html`:

```html
{% extends "base.html" %}

{% block title %}{{ page.title }} — Tesseras{% endblock %}
{% block description %}{{ page.description }}{% endblock %}

{% block content %}
<article>
    <h2>{{ page.title }}</h2>
    {{ page.content | safe }}
</article>
{% endblock %}
```

**Step 3: Write section.html (news listing)**

Create `website/templates/section.html`:

```html
{% extends "base.html" %}

{% block title %}{{ section.title }} — Tesseras{% endblock %}
{% block description %}{{ section.description }}{% endblock %}

{% block content %}
<h2>{{ section.title }}</h2>
{% if section.pages %}
<ul class="news-list">
    {% for page in section.pages %}
    <li>
        <a href="{{ page.permalink }}">{{ page.title }}</a>
        <span class="news-date">{{ page.date | date(format="%Y-%m-%d") }}</span>
        {% if page.description %}
        <p>{{ page.description }}</p>
        {% endif %}
    </li>
    {% endfor %}
</ul>
{% else %}
<p>No posts yet.</p>
{% endif %}
{% endblock %}
```

**Step 4: Commit**

```bash
git add website/templates/
git commit -m "feat(website): add page, section, and index templates"
```

---

### Task 4: Create homepage content (English + Portuguese)

**Files:**
- Create: `website/content/_index.md`
- Create: `website/content/_index.pt-br.md`

**Step 1: Write English homepage**

Create `website/content/_index.md`:

```markdown
+++
title = "Tesseras"
description = "P2P network for preserving human memories across millennia"
+++

Tesseras is a peer-to-peer network for preserving human memories across millennia.

## Why Tesseras Exists

Every year, platforms shut down, companies fail, and file formats become unreadable. Personal photos vanish when a cloud service closes. Home videos rot on obsolete media. Letters are lost in abandoned email accounts. Our memories deserve better than depending on any single company, format, or infrastructure.

## How It Works

Each person creates a **tessera** — a self-contained time capsule of memories (photos, audio, video, text) that survives independently.

- **Peer-to-peer** — your tessera is replicated across a network of volunteers, not stored on corporate servers
- **Erasure coding** — your data is split into redundant fragments so it survives individual node failures
- **No company dependency** — the network runs on open protocols, open source software, and mutual aid
- **Self-describing format** — each tessera contains everything needed to decode itself, even centuries from now
- **Open source** — ISC license, built in Rust

## Current Status

Tesseras is in early development (Phase 0 — Foundation). We are building the core tools for creating, verifying, and exporting tesseras offline.

See [Releases](/releases/) for download information.

## Get Involved

- Join the [mailing list](/subscriptions/)
- Browse the [source code](https://git.sr.ht/~ijanc/tesseras)
- Read the [FAQ](/faq/)
- [Contact us](/contact/)
```

**Step 2: Write Portuguese homepage**

Create `website/content/_index.pt-br.md`:

```markdown
+++
title = "Tesseras"
description = "Rede P2P para preservar memórias humanas através dos milênios"
+++

Tesseras é uma rede peer-to-peer para preservar memórias humanas através dos milênios.

## Por Que Tesseras Existe

Todos os anos, plataformas fecham, empresas falham e formatos de arquivo se tornam ilegíveis. Fotos pessoais desaparecem quando um serviço de nuvem encerra. Vídeos caseiros apodrecem em mídias obsoletas. Cartas se perdem em contas de e-mail abandonadas. Nossas memórias merecem mais do que depender de qualquer empresa, formato ou infraestrutura.

## Como Funciona

Cada pessoa cria uma **tessera** — uma cápsula do tempo autocontida de memórias (fotos, áudio, vídeo, texto) que sobrevive independentemente.

- **Peer-to-peer** — sua tessera é replicada através de uma rede de voluntários, não armazenada em servidores corporativos
- **Codificação por apagamento** — seus dados são divididos em fragmentos redundantes para sobreviver a falhas de nós individuais
- **Sem dependência de empresa** — a rede funciona com protocolos abertos, software livre e ajuda mútua
- **Formato autodescritivo** — cada tessera contém tudo necessário para decodificar a si mesma, mesmo séculos no futuro
- **Código aberto** — licença ISC, construído em Rust

## Status Atual

Tesseras está em desenvolvimento inicial (Fase 0 — Fundação). Estamos construindo as ferramentas básicas para criar, verificar e exportar tesseras offline.

Veja [Lançamentos](/pt-br/releases/) para informações de download.

## Participe

- Entre na [lista de discussão](/pt-br/subscriptions/)
- Navegue pelo [código-fonte](https://git.sr.ht/~ijanc/tesseras)
- Leia o [FAQ](/pt-br/faq/)
- [Fale conosco](/pt-br/contact/)
```

**Step 3: Verify Zola builds**

Run: `cd website && zola build`
Expected: Build succeeds, generates `public/` directory with `index.html`.

**Step 4: Commit**

```bash
git add website/content/_index.md website/content/_index.pt-br.md
git commit -m "feat(website): add homepage content in English and Portuguese"
```

---

### Task 5: Create static pages (FAQ, Releases, Subscriptions, Contact)

**Files:**
- Create: `website/content/pages/_index.md`
- Create: `website/content/pages/about.md`
- Create: `website/content/pages/about.pt-br.md`
- Create: `website/content/pages/faq.md`
- Create: `website/content/pages/faq.pt-br.md`
- Create: `website/content/pages/releases.md`
- Create: `website/content/pages/releases.pt-br.md`
- Create: `website/content/pages/subscriptions.md`
- Create: `website/content/pages/subscriptions.pt-br.md`
- Create: `website/content/pages/contact.md`
- Create: `website/content/pages/contact.pt-br.md`

**Note:** Static pages go under a `pages/` section with `render = false` and `transparent = true` so they appear at root URLs (e.g., `/faq/` not `/pages/faq/`). Alternatively, each page can be its own section. We use the transparent section approach for simplicity.

**Step 1: Create pages section index**

Create `website/content/pages/_index.md`:

```markdown
+++
render = false
transparent = true
+++
```

**Step 2: Write About page (redirect to homepage)**

The About page redirects to the homepage. Create `website/content/pages/about.md`:

```markdown
+++
title = "About"
description = "About the Tesseras project"
redirect_to = "/"
+++
```

Create `website/content/pages/about.pt-br.md`:

```markdown
+++
title = "Sobre"
description = "Sobre o projeto Tesseras"
redirect_to = "/pt-br/"
+++
```

**Step 3: Write FAQ (English)**

Create `website/content/pages/faq.md`:

```markdown
+++
title = "FAQ"
description = "Frequently asked questions about Tesseras"
+++

### What is a tessera?

A tessera is a self-contained time capsule of memories — photos, audio recordings, video, and text — packaged in a format designed to survive independently of any software, company, or infrastructure. The name comes from the small tiles used in Roman mosaics: each piece is simple, but together they form something that endures.

### How does my data survive if my computer dies?

Your tessera is replicated across multiple nodes in the Tesseras peer-to-peer network. It uses erasure coding (Reed-Solomon) to split your data into redundant fragments. Even if several nodes go offline permanently, your tessera can be reconstructed from the remaining fragments.

### Is my data encrypted?

By default, no. Tesseras prioritizes availability over secrecy — the goal is that your memories survive, even if the software to decrypt them doesn't. You can mark individual memories as private (encrypted with AES-256-GCM) or sealed (to be opened after a specific date), but public and circle-visibility memories are stored unencrypted to maximize their chances of long-term survival.

### Do I need to pay anything?

No. The network runs on mutual aid: you store fragments of other people's tesseras, and they store yours. There are no tokens, no blockchain, no subscription fees. The only cost is the storage space you contribute to the network.

### What platforms does it run on?

Tesseras runs on Linux, macOS, FreeBSD, OpenBSD, Windows, Android, and iOS. There's also a browser-based viewer and support for low-power IoT devices (ESP32) as passive storage nodes.

### How is this different from IPFS, Filecoin, or Arweave?

Tesseras is designed specifically for personal memory preservation, not general-purpose file storage. Key differences:

- **No cryptocurrency or tokens** — incentives are based on bilateral reciprocity, not financial markets
- **Self-describing format** — each tessera includes instructions for decoding itself in multiple languages, so it can be understood centuries from now without any special software
- **Availability over secrecy** — most data is stored unencrypted to maximize long-term survival
- **Simplest possible media formats** — JPEG, WAV, WebM, plain text — chosen for durability, not features

### What media formats are supported?

- **Photos:** JPEG
- **Audio:** WAV PCM
- **Video:** WebM
- **Text:** UTF-8 plain text

These formats were chosen for maximum longevity and widespread support.

### Can I export my tessera?

Yes. A tessera is a standard directory of files. You can copy it to a USB drive, burn it to optical media, or print the text portions. The format is designed to be readable without any special software.
```

**Step 4: Write FAQ (Portuguese)**

Create `website/content/pages/faq.pt-br.md`:

```markdown
+++
title = "FAQ"
description = "Perguntas frequentes sobre o Tesseras"
+++

### O que é uma tessera?

Uma tessera é uma cápsula do tempo autocontida de memórias — fotos, gravações de áudio, vídeo e texto — empacotada em um formato projetado para sobreviver independentemente de qualquer software, empresa ou infraestrutura. O nome vem das pequenas peças usadas em mosaicos romanos: cada peça é simples, mas juntas formam algo que perdura.

### Como meus dados sobrevivem se meu computador morrer?

Sua tessera é replicada em múltiplos nós na rede peer-to-peer do Tesseras. Utiliza codificação por apagamento (Reed-Solomon) para dividir seus dados em fragmentos redundantes. Mesmo que vários nós fiquem offline permanentemente, sua tessera pode ser reconstruída a partir dos fragmentos restantes.

### Meus dados são criptografados?

Por padrão, não. O Tesseras prioriza disponibilidade sobre sigilo — o objetivo é que suas memórias sobrevivam, mesmo que o software para descriptografá-las não exista mais. Você pode marcar memórias individuais como privadas (criptografadas com AES-256-GCM) ou seladas (para serem abertas após uma data específica), mas memórias públicas e de círculo são armazenadas sem criptografia para maximizar suas chances de sobrevivência a longo prazo.

### Preciso pagar alguma coisa?

Não. A rede funciona com ajuda mútua: você armazena fragmentos das tesseras de outras pessoas, e elas armazenam as suas. Não há tokens, blockchain ou taxas de assinatura. O único custo é o espaço de armazenamento que você contribui para a rede.

### Em quais plataformas funciona?

Tesseras funciona em Linux, macOS, FreeBSD, OpenBSD, Windows, Android e iOS. Também há um visualizador no navegador e suporte para dispositivos IoT de baixo consumo (ESP32) como nós de armazenamento passivo.

### Qual a diferença do IPFS, Filecoin ou Arweave?

Tesseras é projetado especificamente para preservação de memórias pessoais, não armazenamento de arquivos de propósito geral. Diferenças principais:

- **Sem criptomoeda ou tokens** — incentivos são baseados em reciprocidade bilateral, não mercados financeiros
- **Formato autodescritivo** — cada tessera inclui instruções para decodificar a si mesma em múltiplos idiomas, para que possa ser compreendida séculos no futuro sem nenhum software especial
- **Disponibilidade sobre sigilo** — a maioria dos dados é armazenada sem criptografia para maximizar a sobrevivência a longo prazo
- **Formatos de mídia mais simples possíveis** — JPEG, WAV, WebM, texto puro — escolhidos por durabilidade, não recursos

### Quais formatos de mídia são suportados?

- **Fotos:** JPEG
- **Áudio:** WAV PCM
- **Vídeo:** WebM
- **Texto:** UTF-8 texto puro

Esses formatos foram escolhidos por máxima longevidade e amplo suporte.

### Posso exportar minha tessera?

Sim. Uma tessera é um diretório padrão de arquivos. Você pode copiá-la para um pendrive, gravar em mídia óptica ou imprimir as partes de texto. O formato é projetado para ser legível sem nenhum software especial.
```

**Step 5: Write Releases (English)**

Create `website/content/pages/releases.md`:

```markdown
+++
title = "Releases"
description = "Tesseras software releases and downloads"
+++

No releases yet. Tesseras is in early development (Phase 0).

### Release Format

When available, releases will include:

| File | Description |
|------|-------------|
| `tesseras-X.Y.Z.tar.gz` | Source tarball |
| `tesseras-X.Y.Z.tar.gz.sig` | Signify signature |
| `SHA256` | BLAKE3 checksums |
| `CHANGELOG.md` | What changed |

Releases follow [Semantic Versioning](https://semver.org/). Tarballs are signed with [signify](https://man.openbsd.org/signify).

### Verifying a Release

```
signify -Vep tesseras.pub -m tesseras-X.Y.Z.tar.gz -x tesseras-X.Y.Z.tar.gz.sig
b3sum -c SHA256
```
```

**Step 6: Write Releases (Portuguese)**

Create `website/content/pages/releases.pt-br.md`:

```markdown
+++
title = "Lançamentos"
description = "Lançamentos e downloads do software Tesseras"
+++

Nenhum lançamento ainda. Tesseras está em desenvolvimento inicial (Fase 0).

### Formato de Lançamento

Quando disponíveis, os lançamentos incluirão:

| Arquivo | Descrição |
|---------|-----------|
| `tesseras-X.Y.Z.tar.gz` | Tarball com código-fonte |
| `tesseras-X.Y.Z.tar.gz.sig` | Assinatura signify |
| `SHA256` | Checksums BLAKE3 |
| `CHANGELOG.md` | O que mudou |

Os lançamentos seguem [Versionamento Semântico](https://semver.org/). Tarballs são assinados com [signify](https://man.openbsd.org/signify).

### Verificando um Lançamento

```
signify -Vep tesseras.pub -m tesseras-X.Y.Z.tar.gz -x tesseras-X.Y.Z.tar.gz.sig
b3sum -c SHA256
```
```

**Step 7: Write Subscriptions (English)**

Create `website/content/pages/subscriptions.md`:

```markdown
+++
title = "Subscriptions"
description = "Subscribe to the Tesseras mailing list"
+++

### Mailing List

The Tesseras mailing list is the primary channel for project announcements, development discussion, and community support.

To subscribe, send an email to: **tesseras-subscribe@lists.tesseras.net**

To unsubscribe: **tesseras-unsubscribe@lists.tesseras.net**

[Browse the list archives](https://lists.tesseras.net/tesseras/)

### Atom Feeds

You can also follow the project via Atom feeds in your feed reader:

- [News feed](/atom.xml) — project announcements and updates
```

**Step 8: Write Subscriptions (Portuguese)**

Create `website/content/pages/subscriptions.pt-br.md`:

```markdown
+++
title = "Inscrições"
description = "Inscreva-se na lista de discussão do Tesseras"
+++

### Lista de Discussão

A lista de discussão do Tesseras é o canal principal para anúncios do projeto, discussão de desenvolvimento e suporte da comunidade.

Para se inscrever, envie um e-mail para: **tesseras-subscribe@lists.tesseras.net**

Para cancelar a inscrição: **tesseras-unsubscribe@lists.tesseras.net**

[Navegue pelos arquivos da lista](https://lists.tesseras.net/tesseras/)

### Feeds Atom

Você também pode acompanhar o projeto via feeds Atom no seu leitor de feeds:

- [Feed de notícias](/pt-br/atom.xml) — anúncios e atualizações do projeto
```

**Step 9: Write Contact (English)**

Create `website/content/pages/contact.md`:

```markdown
+++
title = "Contact"
description = "Contact the Tesseras project"
+++

### Mailing List

The best way to reach the project is through the [mailing list](/subscriptions/).

### Source Code

- [SourceHut](https://git.sr.ht/~ijanc/tesseras) (primary)
- [GitHub](https://github.com/ijanc/tesseras) (mirror)

### Resources

- [Book](https://book.tesseras.net) — user documentation (coming soon)
- [Atom feed](/atom.xml) — project news
```

**Step 10: Write Contact (Portuguese)**

Create `website/content/pages/contact.pt-br.md`:

```markdown
+++
title = "Contato"
description = "Entre em contato com o projeto Tesseras"
+++

### Lista de Discussão

A melhor forma de contatar o projeto é através da [lista de discussão](/pt-br/subscriptions/).

### Código-Fonte

- [SourceHut](https://git.sr.ht/~ijanc/tesseras) (primário)
- [GitHub](https://github.com/ijanc/tesseras) (espelho)

### Recursos

- [Livro](https://book.tesseras.net) — documentação para usuários (em breve)
- [Feed Atom](/pt-br/atom.xml) — notícias do projeto
```

**Step 11: Verify Zola builds with all pages**

Run: `cd website && zola build`
Expected: Build succeeds. Check `public/` for: `faq/index.html`, `releases/index.html`, `subscriptions/index.html`, `contact/index.html`, and their `pt-br/` equivalents.

**Step 12: Commit**

```bash
git add website/content/pages/
git commit -m "feat(website): add FAQ, Releases, Subscriptions, and Contact pages"
```

---

### Task 6: Create news section with first post

**Files:**
- Create: `website/content/news/_index.md`
- Create: `website/content/news/_index.pt-br.md`
- Create: `website/content/news/2026-02-13-hello-world.md`
- Create: `website/content/news/2026-02-13-hello-world.pt-br.md`

**Step 1: Write news section index (English)**

Create `website/content/news/_index.md`:

```markdown
+++
title = "News"
description = "Tesseras project news and announcements"
sort_by = "date"
generate_feeds = true
+++
```

**Step 2: Write news section index (Portuguese)**

Create `website/content/news/_index.pt-br.md`:

```markdown
+++
title = "Notícias"
description = "Notícias e anúncios do projeto Tesseras"
sort_by = "date"
generate_feeds = true
+++
```

**Step 3: Write first news post (English)**

Create `website/content/news/2026-02-13-hello-world.md`:

```markdown
+++
title = "Hello, World"
date = 2026-02-13
description = "Introducing the Tesseras project — a P2P network for preserving human memories."
+++

Today we're announcing the Tesseras project: a peer-to-peer network for preserving human memories across millennia.

Tesseras is built on a simple idea — your photos, recordings, and writings deserve to outlast any company, platform, or file format. Each person creates a tessera, a self-contained time capsule that the network keeps alive through mutual aid and redundancy.

The project is in its earliest stage. We're building the foundation: tools to create, verify, and export tesseras offline. The network layer, replication, and apps will follow.

If this mission resonates with you, [join the mailing list](/subscriptions/) or browse the [source code](https://git.sr.ht/~ijanc/tesseras).
```

**Step 4: Write first news post (Portuguese)**

Create `website/content/news/2026-02-13-hello-world.pt-br.md`:

```markdown
+++
title = "Olá, Mundo"
date = 2026-02-13
description = "Apresentando o projeto Tesseras — uma rede P2P para preservar memórias humanas."
+++

Hoje anunciamos o projeto Tesseras: uma rede peer-to-peer para preservar memórias humanas através dos milênios.

Tesseras é construído sobre uma ideia simples — suas fotos, gravações e escritos merecem sobreviver a qualquer empresa, plataforma ou formato de arquivo. Cada pessoa cria uma tessera, uma cápsula do tempo autocontida que a rede mantém viva através de ajuda mútua e redundância.

O projeto está em seu estágio mais inicial. Estamos construindo a fundação: ferramentas para criar, verificar e exportar tesseras offline. A camada de rede, replicação e aplicativos virão em seguida.

Se essa missão ressoa com você, [entre na lista de discussão](/pt-br/subscriptions/) ou navegue pelo [código-fonte](https://git.sr.ht/~ijanc/tesseras).
```

**Step 5: Verify Zola builds with news section**

Run: `cd website && zola build`
Expected: Build succeeds. Check `public/news/index.html` lists the post, `public/news/hello-world/index.html` exists, and `public/news/atom.xml` is generated.

**Step 6: Commit**

```bash
git add website/content/news/
git commit -m "feat(website): add news section with first post"
```

---

### Task 7: Add news post template

**Files:**
- Create: `website/templates/news-post.html`
- Modify: `website/content/news/_index.md` (add `page_template`)

**Step 1: Write news post template**

Create `website/templates/news-post.html`:

```html
{% extends "base.html" %}

{% block title %}{{ page.title }} — Tesseras{% endblock %}
{% block description %}{{ page.description }}{% endblock %}

{% block content %}
<article>
    <h2>{{ page.title }}</h2>
    <p class="news-date">{{ page.date | date(format="%Y-%m-%d") }}</p>
    {{ page.content | safe }}
</article>
{% endblock %}
```

**Step 2: Update news section to use the template**

Add `page_template = "news-post.html"` to the front matter in `website/content/news/_index.md`:

```markdown
+++
title = "News"
description = "Tesseras project news and announcements"
sort_by = "date"
generate_feeds = true
page_template = "news-post.html"
+++
```

Also update `website/content/news/_index.pt-br.md`:

```markdown
+++
title = "Notícias"
description = "Notícias e anúncios do projeto Tesseras"
sort_by = "date"
generate_feeds = true
page_template = "news-post.html"
+++
```

**Step 3: Verify Zola builds**

Run: `cd website && zola build`
Expected: Build succeeds. News post pages use the news-post template.

**Step 4: Commit**

```bash
git add website/templates/news-post.html website/content/news/_index.md website/content/news/_index.pt-br.md
git commit -m "feat(website): add dedicated news post template"
```

---

### Task 8: Integration test and refinement

**Step 1: Run Zola serve and visually inspect**

Run: `cd website && zola serve`
Expected: Site accessible at `http://127.0.0.1:1111`.

**Step 2: Test checklist**

Manually verify in a browser:

- [ ] Homepage loads at `/` with English content
- [ ] Portuguese homepage loads at `/pt-br/`
- [ ] All nav links work (About, News, Releases, FAQ, Subscriptions, Contact)
- [ ] Portuguese nav links work
- [ ] Language switcher toggles between English and Portuguese
- [ ] News listing shows the hello-world post
- [ ] News post page renders correctly
- [ ] Atom feed at `/news/atom.xml` is valid
- [ ] Portuguese Atom feed at `/pt-br/news/atom.xml` exists
- [ ] Footer links work
- [ ] CSS renders: warm cream background, centered content, correct typography
- [ ] No JavaScript in page source

**Step 3: Fix any issues found**

Common things to fix:
- Navigation link paths (Zola's internal linking can be tricky with transparent sections)
- Language switcher URL generation
- Missing hreflang tags (may need to add manually to base.html)

**Step 4: Run zola check**

Run: `cd website && zola check`
Expected: No errors or broken links.

**Step 5: Commit any fixes**

```bash
git add website/
git commit -m "fix(website): resolve integration issues from testing"
```

---

### Task 9: Clean up and final commit

**Step 1: Remove any .gitkeep files**

```bash
find website/ -name .gitkeep -delete
```

**Step 2: Verify final build**

Run: `cd website && zola build`
Expected: Clean build, no warnings.

**Step 3: Check output size**

Run: `du -sh website/public/`
Expected: Small (under 500KB).

**Step 4: Final commit if needed**

```bash
git add website/
git commit -m "chore(website): clean up scaffolding files"
```

---

### Summary

| Task | Description | Key files |
|------|-------------|-----------|
| 1 | Scaffold Zola + config | `config.toml` |
| 2 | Base template + CSS | `base.html`, `style.css` |
| 3 | Page templates | `index.html`, `page.html`, `section.html` |
| 4 | Homepage content | `_index.md`, `_index.pt-br.md` |
| 5 | Static pages | FAQ, Releases, Subscriptions, Contact (en + pt-br) |
| 6 | News section + first post | `news/` directory |
| 7 | News post template | `news-post.html` |
| 8 | Integration test | Manual browser testing |
| 9 | Clean up | Remove scaffolding |
