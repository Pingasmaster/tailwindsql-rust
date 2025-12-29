(() => {
  const ITEMS_PER_PAGE = 10;
  const classNameInput = document.getElementById('pg-className');
  const renderAsSelect = document.getElementById('pg-renderAs');
  const joinToggle = document.getElementById('pg-join-toggle');
  const joinBlock = document.getElementById('pg-join-block');
  const joinTableInput = document.getElementById('pg-join-table');
  const joinOnInput = document.getElementById('pg-join-on');
  const joinSelectInput = document.getElementById('pg-join-select');
  const joinTypeSelect = document.getElementById('pg-join-type');
  const dbClose = document.getElementById('pg-db-close');
  const sqlBlock = document.getElementById('pg-sql-block');
  const sqlCode = document.getElementById('pg-sql');
  const paramsSpan = document.getElementById('pg-params');
  const countDiv = document.getElementById('pg-count');
  const resultsDiv = document.getElementById('pg-results');
  const exampleButtons = Array.from(document.querySelectorAll('.pg-example'));

  if (!classNameInput || !renderAsSelect || !joinToggle || !joinBlock || !resultsDiv) {
    return;
  }

  const inactiveExampleClass =
    'pg-example px-2 sm:px-3 py-1 rounded-full text-xs font-mono transition-all whitespace-nowrap bg-white/5 text-slate-400 border border-white/10 hover:bg-white/10 hover:text-slate-300';
  const activeExampleClass =
    'pg-example px-2 sm:px-3 py-1 rounded-full text-xs font-mono transition-all whitespace-nowrap bg-cyan-500/20 text-cyan-400 border border-cyan-500/30';

  const state = {
    className: classNameInput.value.trim(),
    renderAs: renderAsSelect.value,
    join: {
      enabled: false,
      table: joinTableInput ? joinTableInput.value.trim() : 'posts',
      on: joinOnInput ? joinOnInput.value.trim() : 'id-author_id',
      select: joinSelectInput ? joinSelectInput.value.trim() : 'title',
      type: joinTypeSelect ? joinTypeSelect.value : 'left',
    },
    result: null,
    loading: false,
    currentPage: 1,
    debounceId: null,
  };

  function updateInputWidth(input, min, max) {
    if (!input) return;
    const length = Math.max(min, Math.min((input.value || '').length + 1, max));
    input.style.width = `${length}ch`;
  }

  function setJoinEnabled(enabled) {
    state.join.enabled = enabled;
    if (joinBlock) {
      joinBlock.classList.toggle('hidden', !enabled);
    }
    if (dbClose) {
      dbClose.textContent = enabled ? '>' : '/>';
    }

    const symbol = joinToggle.querySelector('span');
    if (symbol) {
      symbol.textContent = enabled ? '-' : '+';
    }
    joinToggle.className =
      'px-2.5 sm:px-3 py-1.5 rounded-lg text-xs font-medium transition-all flex items-center gap-1.5 sm:gap-2 whitespace-nowrap ' +
      (enabled
        ? 'bg-purple-500/20 text-purple-400 border border-purple-500/30'
        : 'bg-white/5 text-slate-400 border border-white/10 hover:bg-white/10');
  }

  function updateExampleButtons() {
    exampleButtons.forEach((button) => {
      const query = button.dataset.query || '';
      const join = button.dataset.join || '';
      const joinEnabled = Boolean(join);
      const matches =
        state.className === query &&
        (joinEnabled ? state.join.enabled : !state.join.enabled);
      button.className = matches ? activeExampleClass : inactiveExampleClass;
    });
  }

  function scheduleFetch() {
    clearTimeout(state.debounceId);
    state.debounceId = setTimeout(fetchData, 300);
  }

  async function fetchData() {
    if (!state.className.startsWith('db-')) {
      state.result = { error: 'Query must start with "db-"' };
      render();
      return;
    }

    state.loading = true;
    render();

    try {
      let url = `/api/query?className=${encodeURIComponent(state.className)}`;
      if (state.join.enabled && state.join.table) {
        const joinParam = `${state.join.table}:${state.join.on}:${state.join.select}:${state.join.type}`;
        url += `&join=${encodeURIComponent(joinParam)}`;
      }
      const response = await fetch(url);
      const data = await response.json();
      state.result = data;
      state.currentPage = 1;
    } catch (error) {
      state.result = { error: error instanceof Error ? error.message : 'Failed to fetch data' };
    } finally {
      state.loading = false;
      render();
    }
  }

  function render() {
    updateInputWidth(classNameInput, 12, 30);
    updateInputWidth(joinTableInput, 5, 15);
    updateInputWidth(joinOnInput, 10, 20);
    updateInputWidth(joinSelectInput, 5, 15);
    updateExampleButtons();

    if (sqlBlock) {
      const hasQuery = state.result && state.result.query;
      sqlBlock.classList.toggle('hidden', !hasQuery);
      if (hasQuery) {
        sqlCode.textContent = state.result.query;
        if (state.result.params && state.result.params.length > 0) {
          paramsSpan.textContent = `[${state.result.params.join(', ')}]`;
        } else {
          paramsSpan.textContent = '';
        }
      }
    }

    if (countDiv) {
      if (state.result && typeof state.result.count === 'number' && !state.result.error) {
        const count = state.result.count;
        countDiv.textContent = `${count} result${count !== 1 ? 's' : ''}`;
        if (count > ITEMS_PER_PAGE) {
          countDiv.textContent += ` (showing ${ITEMS_PER_PAGE} per page)`;
        }
        countDiv.classList.remove('hidden');
      } else {
        countDiv.classList.add('hidden');
      }
    }

    renderResults();
  }

  function renderResults() {
    if (state.loading) {
      resultsDiv.innerHTML =
        '<div class="flex items-center justify-center h-32">' +
        '<div class="animate-spin rounded-full h-8 w-8 border-2 border-cyan-500 border-t-transparent"></div>' +
        '</div>';
      return;
    }

    if (!state.result) {
      resultsDiv.innerHTML = '';
      return;
    }

    if (state.result.error) {
      resultsDiv.innerHTML =
        `<div class="text-red-400 bg-red-500/10 border border-red-500/20 rounded-lg p-4 font-mono text-sm">` +
        `Warning: ${state.result.error}` +
        `</div>`;
      return;
    }

    const results = state.result.results || [];
    if (!results.length) {
      resultsDiv.innerHTML = '<div class="text-slate-400 italic p-4">No results found</div>';
      return;
    }

    const totalItems = results.length;
    const totalPages = Math.ceil(totalItems / ITEMS_PER_PAGE);
    const startIndex = (state.currentPage - 1) * ITEMS_PER_PAGE;
    const endIndex = startIndex + ITEMS_PER_PAGE;
    const paginated = results.slice(startIndex, endIndex);
    const headers = Object.keys(results[0]);

    let contentHtml = '';

    if (state.renderAs === 'json') {
      contentHtml += `
        <pre class="bg-black/40 text-green-400 p-4 rounded-lg overflow-x-auto text-sm font-mono">${escapeHtml(
          JSON.stringify(paginated, null, 2)
        )}</pre>`;
    } else if (state.renderAs === 'list') {
      contentHtml += '<ul class="space-y-2">';
      paginated.forEach((row) => {
        if (headers.length === 1) {
          contentHtml += `<li class="bg-white/5 rounded-lg p-3 text-slate-300"><div class="break-words">${escapeHtml(
            String(row[headers[0]] ?? '')
          )}</div></li>`;
        } else {
          const items = headers
            .map(
              (header) =>
                `<div class="flex flex-col sm:flex-row sm:items-start gap-1 sm:gap-2 break-words">` +
                `<span class="text-cyan-400 font-medium text-xs sm:text-sm shrink-0 sm:w-32">${escapeHtml(
                  header
                )}:</span>` +
                `<span class="text-slate-300 text-xs sm:text-sm flex-1 min-w-0">${escapeHtml(
                  String(row[header] ?? '')
                )}</span>` +
                `</div>`
            )
            .join('');
          contentHtml += `<li class="bg-white/5 rounded-lg p-3 text-slate-300"><div class="space-y-1.5">${items}</div></li>`;
        }
      });
      contentHtml += '</ul>';
    } else {
      contentHtml += '<div class="overflow-x-auto -mx-3 sm:mx-0">';
      contentHtml += '<table class="w-full border-collapse text-xs sm:text-sm min-w-[500px]">';
      contentHtml += '<thead><tr class="bg-white/5">';
      headers.forEach((header) => {
        contentHtml += `<th class="border border-white/10 px-2 sm:px-3 py-1.5 sm:py-2 text-left font-semibold text-cyan-400 whitespace-nowrap">${escapeHtml(
          header
        )}</th>`;
      });
      contentHtml += '</tr></thead><tbody>';
      paginated.forEach((row) => {
        contentHtml += '<tr class="hover:bg-white/5 transition-colors">';
        headers.forEach((header) => {
          contentHtml += `<td class="border border-white/10 px-2 sm:px-3 py-1.5 sm:py-2 text-slate-300 break-words max-w-[200px] sm:max-w-none">${escapeHtml(
            String(row[header] ?? '')
          )}</td>`;
        });
        contentHtml += '</tr>';
      });
      contentHtml += '</tbody></table></div>';
    }

    if (totalPages > 1) {
      contentHtml += `
        <div class="mt-4 flex flex-row items-center justify-center gap-2">
          <button id="pg-prev" ${
            state.currentPage === 1 ? 'disabled' : ''
          } class="px-2 sm:px-3 py-1.5 rounded-lg text-xs font-medium transition-all bg-white/5 text-slate-400 border border-white/10 hover:bg-white/10 disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap">
            &lt;-
          </button>
          <span class="text-xs text-slate-500 text-center">
            ${state.currentPage}/${totalPages}
            <span class="hidden sm:inline"> (${startIndex + 1}-${Math.min(
              endIndex,
              totalItems
            )} of ${totalItems})</span>
          </span>
          <button id="pg-next" ${
            state.currentPage === totalPages ? 'disabled' : ''
          } class="px-2 sm:px-3 py-1.5 rounded-lg text-xs font-medium transition-all bg-white/5 text-slate-400 border border-white/10 hover:bg-white/10 disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap">
            -&gt;
          </button>
        </div>`;
    }

    resultsDiv.innerHTML = contentHtml;

    const prevBtn = document.getElementById('pg-prev');
    const nextBtn = document.getElementById('pg-next');
    if (prevBtn) {
      prevBtn.addEventListener('click', () => {
        if (state.currentPage > 1) {
          state.currentPage -= 1;
          renderResults();
        }
      });
    }
    if (nextBtn) {
      nextBtn.addEventListener('click', () => {
        if (state.currentPage < totalPages) {
          state.currentPage += 1;
          renderResults();
        }
      });
    }
  }

  function escapeHtml(input) {
    return String(input)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#x27;');
  }

  classNameInput.addEventListener('input', (event) => {
    state.className = event.target.value.trim();
    scheduleFetch();
  });

  renderAsSelect.addEventListener('change', (event) => {
    state.renderAs = event.target.value;
    render();
  });

  joinToggle.addEventListener('click', () => {
    setJoinEnabled(!state.join.enabled);
    scheduleFetch();
  });

  if (joinTableInput) {
    joinTableInput.addEventListener('input', (event) => {
      state.join.table = event.target.value.trim();
      scheduleFetch();
    });
  }
  if (joinOnInput) {
    joinOnInput.addEventListener('input', (event) => {
      state.join.on = event.target.value.trim();
      scheduleFetch();
    });
  }
  if (joinSelectInput) {
    joinSelectInput.addEventListener('input', (event) => {
      state.join.select = event.target.value.trim();
      scheduleFetch();
    });
  }
  if (joinTypeSelect) {
    joinTypeSelect.addEventListener('change', (event) => {
      state.join.type = event.target.value;
      scheduleFetch();
    });
  }

  exampleButtons.forEach((button) => {
    button.addEventListener('click', () => {
      const query = button.dataset.query || '';
      const join = button.dataset.join || '';
      classNameInput.value = query;
      state.className = query;

      if (join) {
        const [table, on, select, type] = join.split(':');
        if (joinTableInput) joinTableInput.value = table || 'posts';
        if (joinOnInput) joinOnInput.value = on || 'id-author_id';
        if (joinSelectInput) joinSelectInput.value = select || 'title';
        if (joinTypeSelect) joinTypeSelect.value = type || 'left';
        state.join.table = table || 'posts';
        state.join.on = on || 'id-author_id';
        state.join.select = select || 'title';
        state.join.type = type || 'left';
        setJoinEnabled(true);
      } else {
        setJoinEnabled(false);
      }

      scheduleFetch();
      render();
    });
  });

  setJoinEnabled(false);
  updateInputWidth(classNameInput, 12, 30);
  updateInputWidth(joinTableInput, 5, 15);
  updateInputWidth(joinOnInput, 10, 20);
  updateInputWidth(joinSelectInput, 5, 15);
  scheduleFetch();
})();
