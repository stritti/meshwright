import { defineConfig } from 'vitepress'

const repo = 'https://github.com/stritti/revmesh'

export default defineConfig({
  title: 'revmesh',
  description: 'Open STL files in SolidCraft as editable designs.',
  lang: 'en-US',
  // Served as a project page: https://stritti.github.io/revmesh/
  base: '/revmesh/',
  cleanUrls: true,
  lastUpdated: true,
  // package.json and node_modules live here too; only Markdown pages are content.
  srcExclude: ['**/node_modules/**'],
  head: [['link', { rel: 'icon', href: '/revmesh/logo.svg', type: 'image/svg+xml' }]],
  themeConfig: {
    logo: '/logo.svg',
    nav: [
      { text: 'Guide', link: '/guide/install' },
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
          { text: 'Roadmap', link: '/guide/roadmap' },
          { text: 'Origin and license', link: '/guide/about' },
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
      message: 'Released under the MIT License. Not affiliated with SolidCraft or mesh2cad.',
    },
  },
})
