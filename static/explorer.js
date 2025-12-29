(() => {
  const root = document.getElementById('explorer-root');
  if (!root) return;

  const state = {
    tables: [],
    activeTable: null,
    loading: true,
    error: null,
  };

  function render() {
    if (state.loading) {
      root.innerHTML =
        '<div class="flex items-center justify-center h-64">' +
        '<div class="animate-spin rounded-full h-8 w-8 border-2 border-cyan-500 border-t-transparent"></div>' +
        '</div>';
      return;
    }

    if (state.error) {
      root.innerHTML = `<div class="text-red-400 text-center py-8">${escapeHtml(state.error)}</div>`;
      return;
    }

    if (!state.tables.length) {
      root.innerHTML = '<div class="text-slate-400 text-center py-8">No tables found.</div>';
      return;
    }

    const current = state.tables.find((table) => table.name === state.activeTable);

    const tableButtons = state.tables
      .map((table) => {
        const active = table.name === state.activeTable;
        const cls =
          'w-full text-left px-2 sm:px-3 py-1.5 sm:py-2 rounded-lg text-xs sm:text-sm font-mono transition-all flex items-center justify-between group ' +
          (active
            ? 'bg-cyan-500/20 text-cyan-400 border border-cyan-500/30'
            : 'text-slate-400 hover:bg-white/5 hover:text-slate-300 border border-transparent');
        return `
          <button data-table="${escapeHtml(table.name)}" class="${cls}">
            <span class="flex items-center gap-1.5 sm:gap-2 truncate">
              <svg class="w-3.5 sm:w-4 h-3.5 sm:h-4 opacity-60 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h18M3 14h18m-9-4v8m-7 0h14a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
              </svg>
              <span class="truncate">${escapeHtml(table.name)}</span>
            </span>
            <span class="text-xs opacity-60 ml-2 flex-shrink-0">${table.rowCount}</span>
          </button>`;
      })
      .join('');

    let columnsHtml = '';
    let dataRowsHtml = '';
    let headersHtml = '';

    if (current) {
      columnsHtml = current.columns
        .map(
          (col) => `
          <div class="flex items-center gap-1 text-xs bg-white/5 border border-white/10 rounded-lg px-1.5 sm:px-2 py-0.5 sm:py-1">
            <span class="text-cyan-400 font-mono">${escapeHtml(col.name)}</span>
            <span class="text-slate-500">:</span>
            <span class="text-orange-400 font-mono">${escapeHtml(col.type)}</span>
          </div>`
        )
        .join('');

      headersHtml = current.columns
        .map(
          (col) => `
          <th class="border border-white/10 px-2 sm:px-3 py-1.5 sm:py-2 text-left font-semibold text-cyan-400 bg-white/5 whitespace-nowrap">
            ${escapeHtml(col.name)}
          </th>`
        )
        .join('');

      dataRowsHtml = current.data
        .map((row) => {
          const cells = current.columns
            .map(
              (col) => `
              <td class="border border-white/10 px-2 sm:px-3 py-1.5 sm:py-2 text-slate-300 font-mono text-xs break-words max-w-[150px] sm:max-w-none">
                ${escapeHtml(formatValue(row[col.name]))}
              </td>`
            )
            .join('');
          return `<tr class="hover:bg-white/5 transition-colors">${cells}</tr>`;
        })
        .join('');
    }

    root.innerHTML = `
      <div class="flex flex-col lg:flex-row gap-3 sm:gap-4 min-h-[400px] sm:min-h-[500px]">
        <div class="lg:w-64 flex-shrink-0">
          <div class="bg-black/40 rounded-xl border border-white/10 overflow-hidden">
            <div class="px-3 sm:px-4 py-2 sm:py-3 border-b border-white/10 bg-white/5">
              <div class="flex items-center gap-2">
                <svg class="w-4 h-4 text-cyan-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4" />
                </svg>
                <span class="text-xs sm:text-sm font-semibold text-slate-300">Tables</span>
              </div>
            </div>
            <div class="p-2">${tableButtons}</div>
          </div>
        </div>
        <div class="flex-1 min-w-0">
          ${
            current
              ? `
          <div class="bg-black/40 rounded-xl border border-white/10 overflow-hidden h-full flex flex-col">
            <div class="px-3 sm:px-4 py-2 sm:py-3 border-b border-white/10 bg-white/5 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-2">
              <div class="flex items-center gap-2 sm:gap-3 flex-wrap">
                <span class="text-base sm:text-lg font-semibold text-white">${escapeHtml(
                  current.name
                )}</span>
                <span class="text-xs text-slate-500 bg-white/10 px-2 py-1 rounded-full">${
                  current.rowCount
                } rows</span>
              </div>
              <code class="text-xs text-purple-400 bg-black/40 px-2 py-1 rounded font-mono break-all">db-${escapeHtml(
                current.name
              )}</code>
            </div>
            <div class="px-3 sm:px-4 py-2 sm:py-3 border-b border-white/10 bg-white/[0.02]">
              <div class="flex flex-wrap gap-1.5 sm:gap-2">${columnsHtml}</div>
            </div>
            <div class="flex-1 overflow-auto p-2 sm:p-4 -mx-2 sm:mx-0">
              <div class="overflow-x-auto">
                <table class="w-full border-collapse text-xs sm:text-sm min-w-[600px]">
                  <thead class="sticky top-0"><tr class="bg-slate-900">${headersHtml}</tr></thead>
                  <tbody>${dataRowsHtml}</tbody>
                </table>
              </div>
            </div>
            <div class="px-3 sm:px-4 py-2 border-t border-white/10 bg-white/[0.02] text-xs text-slate-500">
              Showing ${current.data.length} of ${current.rowCount} rows
            </div>
          </div>`
              : ''
          }
        </div>
      </div>`;
  }

  function formatValue(value) {
    if (value === null || value === undefined) {
      return 'NULL';
    }
    if (typeof value === 'boolean') {
      return value ? 'true' : 'false';
    }
    if (typeof value === 'number') {
      return String(value);
    }
    const str = String(value);
    if (str.length > 50) {
      return str.slice(0, 47) + '...';
    }
    return str;
  }

  function escapeHtml(input) {
    return String(input)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#x27;');
  }

  root.addEventListener('click', (event) => {
    const button = event.target.closest('[data-table]');
    if (!button) return;
    const tableName = button.dataset.table;
    if (!tableName) return;
    state.activeTable = tableName;
    render();
  });

  async function fetchSchema() {
    state.loading = true;
    render();
    try {
      const response = await fetch('/api/schema');
      const data = await response.json();
      state.tables = data.tables || [];
      state.activeTable = state.tables.length ? state.tables[0].name : null;
      state.error = null;
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Failed to fetch schema';
    } finally {
      state.loading = false;
      render();
    }
  }

  fetchSchema();
})();
