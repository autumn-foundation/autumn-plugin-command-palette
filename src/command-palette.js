// The command palette widget pack (autumn#3416).
//
// The keyboard core in autumn-widgets.js moves the active option with the
// arrow keys and clicks it on Enter. This file only:
// - opens and closes the dialog (hotkey, invoker button);
// - asks htmx for the results when the dialog opens;
// - keeps aria-expanded, aria-activedescendant and aria-busy correct;
// - keeps the recent ids in localStorage.
(function () {
  'use strict';

  var PANEL = '.autumn-command-palette-panel';
  var OPTION = '[role="option"]';
  var STORE = 'autumn-command-palette:recent:';
  var ready = new WeakSet();
  // Enter while the list is out of date: run the first option of the next list.
  var pending = new WeakSet();
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

  // The key matches by `e.key`. When `e.key` is not a Latin letter or a
  // digit (a Cyrillic layout, Option+P is π), a letter or a digit matches by
  // `e.code`. A Latin `e.key` never uses `e.code`: on Dvorak, KeyB is `x`.
  function sameKey(e, key) {
    var pressed = (e.key || '').toLowerCase();
    if (pressed === key) return true;
    if (/^[a-z0-9]$/.test(pressed)) return false;
    if (/^[a-z]$/.test(key)) return e.code === 'Key' + key.toUpperCase();
    if (/^[0-9]$/.test(key)) return e.code === 'Digit' + key;
    return false;
  }

  function matches(e, hotkey) {
    if (!sameKey(e, hotkey.key)) return false;
    var ctrl = hotkey.ctrl;
    var meta = hotkey.meta;
    if (hotkey.mod) {
      // `mod` is ⌘ on Apple platforms: Ctrl+K edits text there. On the
      // other platforms `mod` is Ctrl or ⌘, not both.
      if (apple) {
        meta = true;
      } else {
        if (e.ctrlKey === e.metaKey) return false;
        ctrl = e.ctrlKey;
        meta = e.metaKey;
      }
    }
    // For a letter, a digit or an F key, Shift must match. For a symbol,
    // `e.key` already includes Shift.
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
      status: document.getElementById(id + '-status'),
      max: Number(panel.getAttribute('data-command-palette-recent-max')) || 0
    };
  }

  function panelFor(id) {
    return document.querySelector(PANEL + '[data-command-palette="' + CSS.escape(id) + '"]');
  }

  // Make `option` the active option. The keyboard core reads the same
  // attributes.
  function activate(p, option) {
    p.list.querySelectorAll(OPTION).forEach(function (o) {
      o.setAttribute('aria-selected', o === option ? 'true' : 'false');
    });
    if (option) p.input.setAttribute('aria-activedescendant', option.id);
    else p.input.removeAttribute('aria-activedescendant');
  }

  // The list is out of date: Enter must not run an old option.
  function stale(p) {
    p.list.setAttribute('aria-busy', 'true');
    p.input.removeAttribute('aria-activedescendant');
  }

  // The query as the server cuts it: trimmed, 200 characters at most.
  function normalize(value) {
    return Array.from(value.trim()).slice(0, 200).join('').trim();
  }

  // After a swap, set aria-expanded to show if the list has options. Set
  // aria-activedescendant to the selected option. A list for an old query
  // stays busy: the response for the new query comes later.
  function sync(panel) {
    var p = parts(panel);
    if (!p.input || !p.list) return;
    var stamp = p.list.querySelector('template[data-command-palette-query]');
    if (stamp && stamp.getAttribute('data-command-palette-query') !== normalize(p.input.value)) {
      stale(p);
      return;
    }
    p.list.removeAttribute('aria-busy');
    var options = p.list.querySelectorAll(OPTION);
    p.input.setAttribute('aria-expanded', options.length ? 'true' : 'false');
    var active = p.list.querySelector(OPTION + '[aria-selected="true"]');
    activate(p, active);
    if (pending.has(panel)) {
      pending.delete(panel);
      if (active) active.click();
    }
  }

  function open(panel) {
    var p = parts(panel);
    if (!p.dialog || p.dialog.open) return;
    // Show no old results while the new ones load.
    p.input.value = '';
    p.list.replaceChildren();
    if (p.status) p.status.replaceChildren();
    p.input.setAttribute('aria-expanded', 'false');
    stale(p);
    pending.delete(panel);
    p.recent.value = readRecent(p.id).join(',');
    p.dialog.showModal();
    p.input.focus();
    window.htmx.trigger(p.input, 'command-palette-open');
  }

  function toggle(panel) {
    var p = parts(panel);
    if (p.dialog.open) p.dialog.close();
    else open(panel);
  }

  // With JS and htmx on, the button opens the dialog. The link is the
  // no-JS path.
  function upgrade(root) {
    if (!window.htmx) return;
    function each(selector, fn) {
      if (root.matches && root.matches(selector)) fn(root);
      root.querySelectorAll(selector).forEach(fn);
    }
    each('[data-command-palette-open]', function (b) {
      if (panelFor(b.getAttribute('data-command-palette-open'))) b.hidden = false;
    });
    each('[data-command-palette-link]', function (a) {
      if (panelFor(a.getAttribute('data-command-palette-link'))) a.hidden = true;
    });
    if (apple) {
      each('[data-command-palette-mod]', function (k) { k.textContent = '⌘'; });
    }
  }

  function init(panel) {
    if (ready.has(panel)) return;
    ready.add(panel);
    var p = parts(panel);
    if (p.input) {
      p.input.addEventListener('input', function () { stale(p); });
    }
    panel.setAttribute('data-command-palette-ready', '');
  }

  document.addEventListener('autumn:init', function (e) {
    var root = e.target;
    if (!root.querySelectorAll) return;
    if (root.matches && root.matches(PANEL)) init(root);
    root.querySelectorAll(PANEL).forEach(init);
    // The invoker and its panel can come in different swaps.
    upgrade(document.documentElement);
    // An htmx swap into the listbox: the target is the listbox.
    var panel = root.closest && root.closest(PANEL);
    if (panel) sync(panel);
  });

  // A failed request: tell the user, and leave the list as it is.
  function failed(e) {
    var panel = e.target.closest && e.target.closest(PANEL);
    if (!panel) return;
    var p = parts(panel);
    pending.delete(panel);
    p.list.removeAttribute('aria-busy');
    if (p.status) p.status.textContent = panel.getAttribute('data-command-palette-error') || '';
  }
  ['htmx:responseError', 'htmx:sendError', 'htmx:timeout', 'htmx:swapError'].forEach(function (name) {
    document.addEventListener(name, failed);
  });

  document.addEventListener('click', function (e) {
    var t = e.target;
    if (!t.closest) return;
    var opener = t.closest('[data-command-palette-open]');
    if (opener) {
      var panel = panelFor(opener.getAttribute('data-command-palette-open'));
      if (panel) open(panel);
      return;
    }
    var option = t.closest(PANEL + ' [data-command-id]');
    if (option) {
      var own = parts(option.closest(PANEL));
      writeRecent(own.id, option.getAttribute('data-command-id'), own.max);
    }
  });

  // A pointer in the list moves the active option, so the mouse and Enter
  // agree.
  document.addEventListener('pointermove', function (e) {
    var option = e.target.closest && e.target.closest(PANEL + ' ' + OPTION);
    if (!option || option.getAttribute('aria-selected') === 'true') return;
    var p = parts(option.closest(PANEL));
    if (!p.list.hasAttribute('aria-busy')) activate(p, option);
  });

  // A click in the panel keeps focus on the combobox.
  document.addEventListener('mousedown', function (e) {
    var panel = e.target.closest && e.target.closest(PANEL);
    if (panel && !e.target.closest('input')) e.preventDefault();
  });

  document.addEventListener('keydown', function (e) {
    if (e.defaultPrevented || e.isComposing) return;
    var t = e.target;
    // Enter before the new list comes: run its first option when it comes.
    if (e.key === 'Enter' && t.closest) {
      var own = t.closest(PANEL);
      if (own) {
        var p = parts(own);
        if (t === p.input && p.list.hasAttribute('aria-busy')) {
          e.preventDefault();
          pending.add(own);
        }
        return;
      }
    }
    if (e.repeat || !window.htmx) return;
    document.querySelectorAll(PANEL).forEach(function (panel) {
      var dialog = panel.closest('dialog');
      if (!dialog || e.defaultPrevented) return;
      var hotkey = parse(panel.getAttribute('data-command-palette-hotkey'));
      if (!matches(e, hotkey)) return;
      var plain = !hotkey.mod && !hotkey.ctrl && !hotkey.meta && !hotkey.alt;
      // A plain key types in a text field. In the open palette it types too.
      if (plain && (editable(t) || dialog.open)) return;
      e.preventDefault();
      toggle(panel);
    });
  });
})();
