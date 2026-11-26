import { defineConfig } from 'vitepress'

const repo = 'https://github.com/stritti/revmesh'

export default defineConfig({
  title: 'revmesh',
  // Served as a project page: https://stritti.github.io/revmesh/
  base: '/revmesh/',
  cleanUrls: true,
  lastUpdated: true,
  // package.json and node_modules live here too; only Markdown pages are content.
  srcExclude: ['**/node_modules/**'],
  head: [['link', { rel: 'icon', href: '/revmesh/logo.svg', type: 'image/svg+xml' }]],
  locales: {
    root: {
      label: 'English',
      lang: 'en-US',
      description: 'Open STL files in SolidCraft as editable designs.',
      themeConfig: {
        logo: '/logo.svg',
        nav: [
          { text: 'Guide', link: '/guide/install' },
          { text: 'Examples', link: '/guide/examples' },
          { text: 'Development', link: '/architecture' },
          { text: 'Releases', link: `${repo}/releases` },
        ],
        sidebar: [
          {
            text: 'Guide',
            items: [
              { text: 'Install', link: '/guide/install' },
              { text: 'Use', link: '/guide/use' },
              { text: 'What auto recognises', link: '/guide/recognition' },
              { text: 'Examples', link: '/guide/examples' },
              { text: 'Roadmap', link: '/guide/roadmap' },
              { text: 'Origin and license', link: '/guide/about' },
              { text: 'Comparable tools', link: '/guide/comparison' },
            ],
          },
          {
            text: 'Development',
            items: [
              { text: 'Architecture', link: '/architecture' },
              { text: 'Design format', link: '/design-format' },
              { text: 'Development', link: '/development' },
              { text: 'Releasing', link: '/releasing' },
            ],
          },
        ],
        socialLinks: [{ icon: 'github', link: repo }],
        editLink: {
          pattern: `${repo}/edit/main/docs/:path`,
          text: 'Edit this page on GitHub',
        },
        search: { provider: 'local' },
        footer: {
          message: 'Released under the MIT License. Not affiliated with SolidCraft.',
        },
      },
    },
    de: {
      label: 'Deutsch',
      lang: 'de-DE',
      description: 'STL-Dateien in SolidCraft als bearbeitbare Konstruktionen öffnen.',
      markdown: {
        container: {
          tipLabel: 'Tipp',
          noteLabel: 'Hinweis',
          infoLabel: 'Info',
          warningLabel: 'Achtung',
          dangerLabel: 'Gefahr',
          detailsLabel: 'Details',
        },
      },
      themeConfig: {
        logo: '/logo.svg',
        nav: [
          { text: 'Anleitung', link: '/de/guide/install' },
          { text: 'Beispiele', link: '/de/guide/examples' },
          { text: 'Entwicklung', link: '/de/architecture' },
          { text: 'Releases', link: `${repo}/releases` },
        ],
        sidebar: [
          {
            text: 'Anleitung',
            items: [
              { text: 'Installation', link: '/de/guide/install' },
              { text: 'Anwendung', link: '/de/guide/use' },
              { text: 'Was auto erkennt', link: '/de/guide/recognition' },
              { text: 'Beispiele', link: '/de/guide/examples' },
              { text: 'Roadmap', link: '/de/guide/roadmap' },
              { text: 'Herkunft und Lizenz', link: '/de/guide/about' },
              { text: 'Vergleichbare Werkzeuge', link: '/de/guide/comparison' },
            ],
          },
          {
            text: 'Entwicklung',
            items: [
              { text: 'Architektur', link: '/de/architecture' },
              { text: 'Design-Format', link: '/de/design-format' },
              { text: 'Entwicklung', link: '/de/development' },
              { text: 'Release-Prozess', link: '/de/releasing' },
            ],
          },
        ],
        socialLinks: [{ icon: 'github', link: repo }],
        editLink: {
          pattern: `${repo}/edit/main/docs/:path`,
          text: 'Diese Seite auf GitHub bearbeiten',
        },
        search: { provider: 'local' },
        footer: {
          message: 'Veröffentlicht unter der MIT-Lizenz. Nicht mit SolidCraft verbunden.',
        },
      },
    },
  },
})
