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
          light: '#9B9890'
        },
        lapis: {
          DEFAULT: '#376399',
          light: '#5A85B5',
          dark: '#2E5580'
        },
        malachite: {
          DEFAULT: '#5B8A5F',
          light: '#6B8F5F',
          dark: '#4A7550'
        },
        amber: {
          DEFAULT: '#D4943A',
          light: '#E0AA5C',
          dark: '#B87D2E'
        },
        cinnabar: {
          DEFAULT: '#C45B52',
          light: '#D47870',
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
