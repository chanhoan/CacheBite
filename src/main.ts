import './lib/styles/tokens.css';
import './lib/styles/global.css';
import { mount } from 'svelte';

import App from './App.svelte';

const target = document.getElementById('app');

if (!target) {
  throw new Error('CacheBite application root was not found');
}

mount(App, { target });
