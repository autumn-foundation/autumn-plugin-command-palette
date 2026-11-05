// The command palette widget pack (autumn#3416).
//
// The keyboard core in autumn-widgets.js moves the active option with the
// arrow keys and clicks it on Enter. This file only:
// - opens and closes the dialog (hotkey, invoker button);
// - asks htmx for the results when the dialog opens;
// - keeps aria-expanded and aria-activedescendant right after a swap;
// - keeps the recent ids in localStorage.
(function () {
  'use strict';

  var PANEL = '.autumn-command-palette-panel';
  var STORE = 'autumn-command-palette:recent:';
  var ready = new WeakSet();
  var apple = /Mac|iPhone|iPad|iPod/.test(
    (navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || ''
  );

  // `mod+shift+p` -> { key: 'p', mod: true, shift: true, ... }.
  function parse(spec) {
    var parts = (spec || 'mod+k').split('+');
    var hotkey = { key: parts.pop(), mod: false, ctrl: false, meta: false, alt: false, shift: false };
    parts.forEach(function (part) { hotkey[part] = true; });
    return hotkey;
  }

  function matches(e, hotkey) {
    if ((e.key || '').toLowerCase() !== hotkey.key) return false;
    var ctrl = hotkey.ctrl, meta = hotkey.meta;
    if (hotkey.mod) {
      // `mod` is Ctrl or Meta, on each platform.
      if (!e.ctrlKey && !e.metaKey) return false;
      if (e.ctrlKey && !ctrl && e.metaKey && !meta) return false;
      ctrl = ctrl || e.ctrlKey;
      meta = meta || e.metaKey;
    }
    // A letter tells Shift apart. A symbol (`?`) already has its Shift.
    var shiftMatters = /^[a-z0-9]$|^f\d+$/.test(hotkey.key);
    return e.ctrlKey === ctrl && e.metaKey === meta && e.altKey === hotkey.alt &&
      (!shiftMatters || e.shiftKey === hotkey.shift);
  }

  function editable(el) {
    return !!el && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName));
  }

  function readRecent(id) {
    try {
      var list = JSON.parse(window.localStorage.getItem(STORE + id) || '[]');
      return Array.isArray(list) ? list.filter(function (x) { return typeof x === 'string'; }) : [];
    } catch (err) {
      return [];
    }
  }

  function writeRecent(id, commandId, max) {
    var list = readRecent(id).filter(function (x) { return x !== commandId; });
    list.unshift(commandId);
    try {
      window.localStorage.setItem(STORE + id, JSON.stringify(list.slice(0, max)));
    } catch (err) {
      // Storage is off (a private window). Recent items are a convenience.
    }
  }

  function parts(panel) {
    var id = panel.getAttribute('data-command-palette');
    return {
      id: id,
      dialog: panel.closest('dialog'),
      input: document.getElementById(id + '-input'),
      recent: document.getElementById(id + '-recent'),
      list: document.getElementById(id + '-list'),
      max: Number(panel.getAttribute('data-command-palette-recent-max')) || 0
    };
  }

  // After a swap: aria-expanded tells if there are options; the active
  // option is the first selected one.
  function sync(panel) {
    var p = parts(panel);
    if (!p.input || !p.list) return;
    var options = p.list.querySelectorAll('[role="option"]');
    p.input.setAttribute('aria-expanded', options.length ? 'true' : 'false');
    var active = p.list.querySelector('[role="option"][aria-selected="true"]');
    if (active) p.input.setAttribute('aria-activedescendant', active.id);
    else p.input.removeAttribute('aria-activedescendant');
  }

  function open(panel) {
    var p = parts(panel);
    if (!p.dialog || p.dialog.open) return;
    p.input.value = '';
    p.recent.value = readRecent(p.id).join(',');
    p.dialog.showModal();
    p.input.focus();
    if (window.htmx) window.htmx.trigger(p.input, 'command-palette-open');
  }

  function toggle(panel) {
    var p = parts(panel);
    if (!p.dialog) return;
    if (p.dialog.open) p.dialog.close();
    else open(panel);
  }

  function init(panel) {
    if (ready.has(panel)) return;
    ready.add(panel);
    var id = panel.getAttribute('data-command-palette');
    // With JS on, the button opens the dialog. The link is the no-JS path.
    document.querySelectorAll('[data-command-palette-open="' + id + '"]').forEach(function (b) {
      b.hidden = false;
    });
    document.querySelectorAll('[data-command-palette-link="' + id + '"]').forEach(function (a) {
      a.hidden = true;
    });
    if (apple) {
      document.querySelectorAll('[data-command-palette-mod]').forEach(function (k) {
        k.textContent = '⌘';
      });
    }
    panel.setAttribute('data-command-palette-ready', '');
  }

  document.addEventListener('autumn:init', function (e) {
    var root = e.target;
    if (!root.querySelectorAll) return;
    if (root.matches && root.matches(PANEL)) init(root);
    root.querySelectorAll(PANEL).forEach(init);
    // An htmx swap into the listbox: the target is the listbox.
    var panel = root.closest && root.closest(PANEL);
    if (panel) sync(panel);
  });

  document.addEventListener('click', function (e) {
    var t = e.target;
    if (!t.closest) return;
    var opener = t.closest('[data-command-palette-open]');
    if (opener) {
      var panel = document.querySelector(
        PANEL + '[data-command-palette="' + opener.getAttribute('data-command-palette-open') + '"]'
      );
      if (panel) open(panel);
      return;
    }
    var option = t.closest(PANEL + ' [data-command-id]');
    if (option) {
      var own = parts(option.closest(PANEL));
      writeRecent(own.id, option.getAttribute('data-command-id'), own.max);
    }
  });

  document.addEventListener('keydown', function (e) {
    if (e.defaultPrevented || e.isComposing) return;
    document.querySelectorAll(PANEL).forEach(function (panel) {
      var p = parts(panel);
      if (!p.dialog || e.defaultPrevented) return;
      var hotkey = parse(panel.getAttribute('data-command-palette-hotkey'));
      if (!matches(e, hotkey)) return;
      var plain = !hotkey.mod && !hotkey.ctrl && !hotkey.meta && !hotkey.alt;
      // A plain key types in a text field. In the open palette it types too.
      if (plain && (editable(e.target) || p.dialog.open)) return;
      e.preventDefault();
      toggle(panel);
    });
  });
})();
