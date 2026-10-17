for (const gallery of document.querySelectorAll('[data-gallery]')) {
  const tabs = [...gallery.querySelectorAll('[role="tab"]')];
  for (const [index, tab] of tabs.entries()) {
    tab.addEventListener('click', () => select(index));
    tab.addEventListener('keydown', event => {
      const next = event.key === 'ArrowRight' ? index + 1 : event.key === 'ArrowLeft' ? index - 1 : null;
      if (next === null) return;
      event.preventDefault();
      const target = (next + tabs.length) % tabs.length;
      select(target);
      tabs[target].focus();
    });
  }

  function select(index) {
    for (const [position, tab] of tabs.entries()) {
      const active = position === index;
      tab.setAttribute('aria-selected', String(active));
      tab.tabIndex = active ? 0 : -1;
      gallery.querySelector(`#${tab.getAttribute('aria-controls')}`).hidden = !active;
    }
  }
}
