const state = { records: [], status: 'all', query: '', sortKey: null, sortDirection: 'asc' }

const elements = {
  body: document.getElementById('ip-table-body'),
  empty: document.getElementById('empty-state'),
  notice: document.getElementById('notice'),
  resultCount: document.getElementById('result-count'),
  search: document.getElementById('search-input'),
  refresh: document.getElementById('refresh-button'),
  dialog: document.getElementById('request-dialog'),
  form: document.getElementById('request-form'),
  requestIp: document.getElementById('request-ip'),
  requestTitle: document.getElementById('request-title'),
  submit: document.getElementById('submit-request'),
  formError: document.getElementById('form-error'),
  usageMode: document.getElementById('usage-mode'),
  usageDateField: document.getElementById('usage-date-field'),
  usageEndDate: document.getElementById('usage-end-date'),
}

function text(value) {
  return value || '--'
}

function todayValue() {
  const now = new Date()
  const offset = now.getTimezoneOffset() * 60_000
  return new Date(now.getTime() - offset).toISOString().slice(0, 10)
}

function formatUsagePeriod(value) {
  if (!value) return 'No end date'
  const [year, month, day] = value.split('-').map(Number)
  const formatted = new Date(year, month - 1, day).toLocaleDateString()
  return value < todayValue() ? `${formatted} (expired)` : `Until ${formatted}`
}

function isExpired(item) {
  return item.status !== 'Unused' && item.usageEndDate && item.usageEndDate < todayValue()
}

function showNotice(message, isError = false) {
  elements.notice.textContent = message
  elements.notice.classList.toggle('error', isError)
  elements.notice.hidden = false
}

function hideNotice() {
  elements.notice.hidden = true
}

function setCounts() {
  document.getElementById('total-count').textContent = state.records.length
  document.getElementById('used-count').textContent = state.records.filter((item) => item.status === 'Used').length
  document.getElementById('unused-count').textContent = state.records.filter((item) => item.requestAvailable).length
  document.getElementById('reserved-count').textContent = state.records.filter((item) => item.status === 'Reserved').length
}

function filteredRecords() {
  const query = state.query.trim().toLowerCase()
  const records = state.records.filter((item) => {
    if (state.status === 'Available' && !item.requestAvailable) return false
    if (state.status !== 'all' && state.status !== 'Available' && item.status !== state.status) return false
    if (!query) return true
    return [item.ipAddress, item.status, item.location, item.responsible, item.environment,
      item.deviceType, item.serialNumber, item.antenna1, item.antenna2, item.usageEndDate, item.notes]
      .filter(Boolean)
      .join(' ')
      .toLowerCase()
      .includes(query)
  })

  if (!state.sortKey) return records

  return records.sort((left, right) => {
    const leftValue = sortValue(left, state.sortKey)
    const rightValue = sortValue(right, state.sortKey)
    if (!leftValue && !rightValue) return left.sortOrder - right.sortOrder
    if (!leftValue) return 1
    if (!rightValue) return -1
    const comparison = leftValue.localeCompare(rightValue, undefined, { numeric: true, sensitivity: 'base' })
    return comparison === 0
      ? left.sortOrder - right.sortOrder
      : state.sortDirection === 'asc' ? comparison : -comparison
  })
}

function sortValue(item, key) {
  if (key === 'antennas') return [item.antenna1, item.antenna2].filter(Boolean).join(' ')
  return item[key] || ''
}

function updateSortHeaders() {
  document.querySelectorAll('[data-sort]').forEach((button) => {
    const isActive = button.dataset.sort === state.sortKey
    const direction = isActive ? state.sortDirection : null
    button.dataset.direction = direction || ''
    button.parentElement.setAttribute('aria-sort', direction === 'asc' ? 'ascending' : direction === 'desc' ? 'descending' : 'none')
    button.querySelector('.sort-indicator').textContent = direction === 'asc' ? '\u2191' : direction === 'desc' ? '\u2193' : '\u2195'
  })
}

function addCell(row, value, className = '') {
  const cell = document.createElement('td')
  cell.textContent = text(value)
  if (className) cell.className = className
  if (value) cell.title = value
  row.appendChild(cell)
  return cell
}

function render() {
  const records = filteredRecords()
  elements.body.replaceChildren()
  for (const item of records) {
    const row = document.createElement('tr')
    row.classList.toggle('usage-expired', Boolean(isExpired(item)))
    addCell(row, item.ipAddress)

    const statusCell = document.createElement('td')
    const status = document.createElement('span')
    const displayStatus = item.status === 'Unused' && !item.requestAvailable ? 'Unavailable' : item.status
    status.className = `status ${displayStatus.toLowerCase()}`
    status.textContent = displayStatus
    statusCell.appendChild(status)
    if (item.requestPending) {
      const pending = document.createElement('span')
      pending.className = 'pending'
      pending.textContent = 'Request pending'
      statusCell.appendChild(document.createElement('br'))
      statusCell.appendChild(pending)
    }
    row.appendChild(statusCell)

    addCell(row, item.location)
    addCell(row, item.responsible)
    addCell(row, item.environment)
    addCell(row, item.deviceType)
    addCell(row, item.serialNumber)
    addCell(row, [item.antenna1, item.antenna2].filter(Boolean).join(', '))
    addCell(row, formatUsagePeriod(item.usageEndDate), isExpired(item) ? 'usage-expired-cell' : '')
    addCell(row, item.notes)

    const actionCell = document.createElement('td')
    if (item.requestAvailable) {
      const button = document.createElement('button')
      button.type = 'button'
      button.className = 'primary-button request-button'
      button.textContent = 'Request IP'
      button.addEventListener('click', () => openRequest(item.ipAddress))
      actionCell.appendChild(button)
    }
    row.appendChild(actionCell)
    elements.body.appendChild(row)
  }

  elements.empty.hidden = records.length > 0
  elements.resultCount.textContent = `${records.length} of ${state.records.length} addresses`
}

async function loadInventory() {
  elements.refresh.disabled = true
  hideNotice()
  try {
    const response = await fetch('/api/v1/guest/ip-addresses', { cache: 'no-store' })
    if (!response.ok) throw new Error(`The IP list could not be loaded. HTTP ${response.status}.`)
    state.records = await response.json()
    setCounts()
    render()
  } catch (error) {
    showNotice(error instanceof Error ? error.message : String(error), true)
  } finally {
    elements.refresh.disabled = false
  }
}

function openRequest(ipAddress) {
  elements.form.reset()
  syncUsageDateField()
  elements.requestIp.value = ipAddress
  elements.requestTitle.textContent = `Request ${ipAddress}`
  elements.formError.hidden = true
  elements.dialog.hidden = false
  document.body.style.overflow = 'hidden'
  elements.form.elements.responsible.focus()
}

function syncUsageDateField() {
  const useDate = elements.usageMode.value === 'date'
  elements.usageDateField.hidden = !useDate
  elements.usageEndDate.disabled = !useDate
  elements.usageEndDate.required = useDate
  if (!useDate) elements.usageEndDate.value = ''
}

function closeRequest() {
  elements.dialog.hidden = true
  document.body.style.overflow = ''
}

async function sendRequest(event) {
  event.preventDefault()
  elements.submit.disabled = true
  elements.formError.hidden = true
  const form = new FormData(elements.form)
  const payload = Object.fromEntries(form.entries())
  payload.requestedBy = String(payload.responsible).trim()
  for (const [key, value] of Object.entries(payload)) {
    if (key !== 'ipAddress' && key !== 'requestedBy' && !String(value).trim()) payload[key] = null
  }

  try {
    const response = await fetch('/api/v1/guest/ip-requests', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    })
    if (!response.ok) {
      const problem = await response.json().catch(() => ({}))
      throw new Error(problem.detail || `The request could not be sent. HTTP ${response.status}.`)
    }
    const receipt = await response.json()
    closeRequest()
    showNotice(`Request sent for ${receipt.ipAddress}. The QA team will review it.`)
    await loadInventory()
    showNotice(`Request sent for ${receipt.ipAddress}. The QA team will review it.`)
  } catch (error) {
    elements.formError.textContent = error instanceof Error ? error.message : String(error)
    elements.formError.hidden = false
  } finally {
    elements.submit.disabled = false
  }
}

elements.search.addEventListener('input', (event) => {
  state.query = event.target.value
  render()
})

document.querySelectorAll('[data-status]').forEach((button) => {
  button.addEventListener('click', () => {
    state.status = button.dataset.status
    document.querySelectorAll('[data-status]').forEach((item) => item.classList.toggle('active', item === button))
    render()
  })
})

document.querySelectorAll('[data-sort]').forEach((button) => {
  button.addEventListener('click', () => {
    const key = button.dataset.sort
    if (state.sortKey === key) {
      state.sortDirection = state.sortDirection === 'asc' ? 'desc' : 'asc'
    } else {
      state.sortKey = key
      state.sortDirection = 'asc'
    }
    updateSortHeaders()
    render()
  })
})

elements.refresh.addEventListener('click', loadInventory)
elements.usageMode.addEventListener('change', syncUsageDateField)
elements.form.addEventListener('submit', sendRequest)
document.getElementById('close-modal').addEventListener('click', closeRequest)
document.getElementById('cancel-request').addEventListener('click', closeRequest)
document.getElementById('modal-backdrop').addEventListener('click', closeRequest)
document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape' && !elements.dialog.hidden) closeRequest()
})

elements.usageEndDate.min = todayValue()
syncUsageDateField()
loadInventory()
