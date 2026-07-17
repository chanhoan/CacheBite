import '@fontsource/ibm-plex-sans/latin-400.css';
import '@fontsource/ibm-plex-sans/latin-500.css';
import '@fontsource/ibm-plex-sans/latin-600.css';
import '@fontsource/ibm-plex-sans/latin-700.css';
import '@fontsource/ibm-plex-mono/latin-400.css';
import '@fontsource/ibm-plex-mono/latin-500.css';
import '@fontsource/ibm-plex-mono/latin-600.css';
import './lib/styles/tokens.css';
import './lib/styles/global.css';
import { mount } from 'svelte';

import App from './App.svelte';

const target = document.getElementById('app');

if (!target) {
  throw new Error('CacheBite application root was not found');
}

mount(App, { target });
