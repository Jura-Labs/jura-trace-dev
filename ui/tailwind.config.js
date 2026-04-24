/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  darkMode: 'class',
  theme: {
    extend: {
      fontFamily: {
        heading: ['Georgia', 'Times New Roman', 'DejaVu Serif', 'Noto Serif', 'serif'],
        body: ['-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'Roboto', 'Oxygen', 'Ubuntu', 'Cantarell', 'sans-serif'],
        mono: ['SF Mono', 'Menlo', 'Monaco', 'Courier New', 'monospace']
      },
      letterSpacing: {
        brand: '0.15em',
        nav: '0.05em',
        heading: '-0.01em',
        button: '0.02em'
      },
      colors: {
        // ===== Jura Trace — Sanctuary Mineral Palette =====
        obsidian: {
          DEFAULT: '#1E2128',
          light: '#272B34',
          dark: '#161920'
        },
        graphite: {
          DEFAULT: '#272B34',
          light: '#2E3340',
          dark: '#1E2128'
        },
        quartz: {
          DEFAULT: '#EDEAE4',
          dark: '#D8D5CE'
        },
        flint: {
          DEFAULT: '#78756D',
          // Darker variant for light-mode secondary text.  Added 2026-04-24
          // after the Lighthouse audit found #78756D gave 4.4:1 contrast on
          // the #FAFAF7 surface — just below WCAG AA 4.5:1.  #5C5A55 yields
          // ~6.7:1 (comfortably above AA, approaches AAA) while staying in
          // the mineral family.  Used as `text-flint-dark` in every place
          // that previously used `text-flint` on a light surface.
          dark: '#5C5A55',
          // Brightened from #9B9890 to #ABA8A0 on 2026-04-24 — Setup Wizard
          // (always-dark modal) was hitting 4.35:1 contrast against the
          // graphite-light panel bg, just below WCAG AA.  #ABA8A0 gives
          // ~5.4:1 there and ~7:1 on the standard obsidian dark surface.
          light: '#ABA8A0'
        },
        lapis: {
          DEFAULT: '#376399',
          // Brightened from #5A85B5 to #7AA0CC on 2026-04-24 — same audit
          // pass.  text-lapis-light buttons in the Setup Wizard were 3.25:1
          // on graphite (passes AA Large but not AA Normal at text-xs).
          // #7AA0CC reaches ~4.7:1 on graphite while still reading clearly
          // as the lapis brand colour.
          light: '#7AA0CC',
          dark: '#2E5580'
        },
        malachite: {
          DEFAULT: '#5B8A5F',
          // Brightened from #6B8F5F to #7DA771 on 2026-04-24 — original
          // gave 4.38:1 contrast on obsidian dark surfaces, just below
          // WCAG AA 4.5:1.  #7DA771 reaches ~5.4:1 on obsidian while
          // staying within the malachite green family.
          light: '#7DA771',
          dark: '#4A7550'
        },
        amber: {
          DEFAULT: '#D4943A',
          light: '#E0AA5C',
          // Darkened from #B87D2E to #8C5F22 on 2026-04-24 — the rc.22
          // light-mode audit found amber-dark on cream surfaces gave only
          // 3.34:1 contrast, failing WCAG AA for normal text.  #8C5F22
          // gives ~5.3:1 on #FAFAF7 and ~4.7:1 on tinted amber/5
          // backgrounds, comfortably above the 4.5:1 threshold while
          // staying recognisably amber.
          dark: '#8C5F22'
        },
        cinnabar: {
          DEFAULT: '#C45B52',
          // Brightened from #D47870 to #DD8C84 on 2026-04-24 — dark-mode
          // cinnabar/10 panel backgrounds gave 4.33:1 contrast against
          // the original cinnabar.light, just below WCAG AA 4.5:1.
          // #DD8C84 reaches ~4.7:1 on cinnabar/10 panels and ~6.0:1 on
          // the standard obsidian dark surface.
          light: '#DD8C84',
          dark: '#A84840'
        },
        // ===== Semantic aliases =====
        surface: {
          light: '#FAFAF7',
          dark: '#1E2128'
        },
        text: {
          light: '#3A3832',
          dark: '#EDEAE4'
        },
        border: {
          light: '#E8E6E0',
          dark: '#33312E'
        }
      },
      maxWidth: {
        'content': '56rem', // ~896px — Sanctuary content width
      },
      typography: (theme) => ({
        DEFAULT: {
          css: {
            maxWidth: 'none',
            color: theme('colors.text.light'),
            lineHeight: '1.7',
            a: {
              color: theme('colors.lapis.DEFAULT'),
              '&:hover': {
                color: theme('colors.lapis.dark'),
              },
            },
          },
        },
        dark: {
          css: {
            color: theme('colors.text.dark'),
            a: {
              color: theme('colors.lapis.light'),
            },
          },
        },
      }),
    },
  },
  plugins: [
    require('@tailwindcss/typography'),
  ],
}
