import { useCallback, useEffect, useMemo, useState, type FormEvent } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import {
  Activity,
  AlertTriangle,
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  Check,
  Circle,
  Clock3,
  Pencil,
  RefreshCw,
  Search,
  X,
} from 'lucide-react'

type IpStatus = 'Unused' | 'Used' | 'Reserved'
type IpStatusFilter = 'all' | IpStatus
type IpSortField = 'location' | 'responsible' | 'deviceType' | 'usageEndDate' | 'antennas'
type SortDirection = 'asc' | 'desc'

type IpInventoryRecord = {
  ipAddress: string
  sortOrder: number
  status: IpStatus
  location: string | null
  responsible: string | null
  environment: string | null
  deviceType: string | null
  serialNumber: string | null
  antenna1: string | null
  antenna2: string | null
  notes: string | null
  usageEndDate: string | null
  isOnline: boolean | null
  lastPingAt: string | null
  rowVersion: number
  updatedBy: string
  updatedAt: string
}

type IpRequestRecord = {
  id: string
  ipAddress: string
  requestedBy: string
  location: string | null
  responsible: string | null
  environment: string | null
  deviceType: string | null
  serialNumber: string | null
  antenna1: string | null
  antenna2: string | null
  notes: string | null
  usageEndDate: string | null
  status: 'Pending' | 'Approved' | 'Rejected'
  decisionBy: string | null
  decisionReason: string | null
  decidedAt: string | null
  rowVersion: number
  createdAt: string
  updatedAt: string
}

type IpDraft = {
  status: IpStatus
  location: string
  responsible: string
  environment: string
  deviceType: string
  serialNumber: string
  antenna1: string
  antenna2: string
  notes: string
  usageEndDate: string
}

type EditTarget =
  | { kind: 'inventory'; ip: IpInventoryRecord }
  | { kind: 'request'; ip: IpInventoryRecord; request: IpRequestRecord }

type IpManagementPageProps = {
  onActivity: (message: string) => void
  onPendingRequestCountChange: (count: number) => void
}

type IpPingScanResult = {
  totalCount: number
  onlineCount: number
  offlineCount: number
  startedAt: string
  completedAt: string
}

type IpPingResult = {
  ipAddress: string
  isOnline: boolean
  checkedAt: string
}

const LOCATION_OPTIONS = ['QA Lab', 'CBC Trailer', 'Automotive Lab', 'Arctic']
const ENVIRONMENT_OPTIONS = ['', 'Production', 'Development', 'Testing/Lab', 'Staging']
const DEVICE_OPTIONS = [
  'Alloy',
  'NetR',
  'R-Series',
  'BD',
  'BX',
  'u-blox',
  'Septentrio',
  'NovAtel',
  'Leica',
  'Unicore',
  'Allystar',
  'Topcon',
  'Other',
]

function createMockInventory(): IpInventoryRecord[] {
  const used = new Set([6, 11, 12, 13, 14, 15, 16, 17, 22, 25, 26, 37, 39, 40, 45, 51, 55, 57, 58, 59])
  const reserved = new Set([3, 4, 18, 19, 20])
  return Array.from({ length: 254 }, (_, index) => {
    const host = index + 1
    const status: IpStatus = used.has(host) ? 'Used' : reserved.has(host) ? 'Reserved' : 'Unused'
    return {
      ipAddress: `10.2.155.${host}`,
      sortOrder: host,
      status,
      location: status === 'Unused' ? null : host % 3 === 0 ? 'CBC Trailer' : 'QA Lab',
      responsible: status === 'Used' ? (host % 2 === 0 ? 'QA Team' : 'Receiver Team') : null,
      environment: status === 'Used' ? (host % 2 === 0 ? 'Testing/Lab' : 'Development') : null,
      deviceType: status === 'Used' ? (host % 2 === 0 ? 'Alloy' : 'BD') : null,
      serialNumber: status === 'Used' ? `SN-${24000 + host}` : null,
      antenna1: status === 'Used' ? `ANT-${100 + host}` : null,
      antenna2: status === 'Used' && host % 4 === 0 ? `ANT-${300 + host}` : null,
      notes: host === 11 ? 'Reference receiver' : null,
      usageEndDate: status === 'Unused' ? null : host % 5 === 0 ? '2026-09-30' : null,
      isOnline: status === 'Used' || host === 24,
      lastPingAt: '2026-09-07T12:45:00Z',
      rowVersion: 1,
      updatedBy: 'DEMO\\user',
      updatedAt: '2026-09-07T12:45:00Z',
    }
  })
}

function createMockRequests(): IpRequestRecord[] {
  return [
    {
      id: 'request-demo-1',
      ipAddress: '10.2.155.30',
      requestedBy: 'Anna Smith',
      location: 'QA Lab',
      responsible: 'Receiver Team',
      environment: 'Testing/Lab',
      deviceType: 'Alloy',
      serialNumber: 'SN-28104',
      antenna1: 'ANT-104',
      antenna2: 'ANT-212',
      notes: 'Needed for firmware validation.',
      usageEndDate: '2026-10-15',
      status: 'Pending',
      decisionBy: null,
      decisionReason: null,
      decidedAt: null,
      rowVersion: 1,
      createdAt: '2026-09-07T11:18:00Z',
      updatedAt: '2026-09-07T11:18:00Z',
    },
    {
      id: 'request-demo-2',
      ipAddress: '10.2.155.31',
      requestedBy: 'Mark Taylor',
      location: 'CBC Trailer',
      responsible: 'Automotive QA',
      environment: 'Development',
      deviceType: 'R-Series',
      serialNumber: '',
      antenna1: 'ANT-118',
      antenna2: '',
      notes: 'Temporary setup.',
      usageEndDate: null,
      status: 'Pending',
      decisionBy: null,
      decisionReason: null,
      decidedAt: null,
      rowVersion: 1,
      createdAt: '2026-09-07T10:06:00Z',
      updatedAt: '2026-09-07T10:06:00Z',
    },
  ]
}

function valueOrEmpty(value: string | null) {
  return value || ''
}

function draftFromIp(ip: IpInventoryRecord): IpDraft {
  return {
    status: ip.status,
    location: valueOrEmpty(ip.location),
    responsible: valueOrEmpty(ip.responsible),
    environment: valueOrEmpty(ip.environment),
    deviceType: valueOrEmpty(ip.deviceType),
    serialNumber: valueOrEmpty(ip.serialNumber),
    antenna1: valueOrEmpty(ip.antenna1),
    antenna2: valueOrEmpty(ip.antenna2),
    notes: valueOrEmpty(ip.notes),
    usageEndDate: valueOrEmpty(ip.usageEndDate),
  }
}

function draftFromRequest(request: IpRequestRecord): IpDraft {
  return {
    status: 'Used',
    location: valueOrEmpty(request.location),
    responsible: valueOrEmpty(request.responsible),
    environment: valueOrEmpty(request.environment),
    deviceType: valueOrEmpty(request.deviceType),
    serialNumber: valueOrEmpty(request.serialNumber),
    antenna1: valueOrEmpty(request.antenna1),
    antenna2: valueOrEmpty(request.antenna2),
    notes: valueOrEmpty(request.notes),
    usageEndDate: valueOrEmpty(request.usageEndDate),
  }
}

function formatError(error: unknown) {
  return error instanceof Error ? error.message : String(error)
}

function formatDate(value: string | null) {
  if (!value) return 'Not available'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString()
}

function todayValue() {
  const now = new Date()
  const offset = now.getTimezoneOffset() * 60_000
  return new Date(now.getTime() - offset).toISOString().slice(0, 10)
}

function formatUsagePeriod(value: string | null) {
  if (!value) return 'No end date'
  const [year, month, day] = value.split('-').map(Number)
  const formatted = new Date(year, month - 1, day).toLocaleDateString()
  return value < todayValue() ? `${formatted} (expired)` : `Until ${formatted}`
}

function isUsageExpired(item: Pick<IpInventoryRecord, 'status' | 'usageEndDate'>) {
  return item.status !== 'Unused' && Boolean(item.usageEndDate && item.usageEndDate < todayValue())
}

function formatAntennas(antenna1: string | null, antenna2: string | null) {
  return [antenna1, antenna2].filter(Boolean).join(', ') || '--'
}

function inventorySortValue(item: IpInventoryRecord, field: IpSortField) {
  if (field === 'antennas') return [item.antenna1, item.antenna2].filter(Boolean).join(' ')
  return item[field] || ''
}

export default function IpManagementPage({ onActivity, onPendingRequestCountChange }: IpManagementPageProps) {
  const [activeTab, setActiveTab] = useState<'inventory' | 'requests'>('inventory')
  const [inventory, setInventory] = useState<IpInventoryRecord[]>([])
  const [requests, setRequests] = useState<IpRequestRecord[]>([])
  const [query, setQuery] = useState('')
  const [statusFilter, setStatusFilter] = useState<IpStatusFilter>('all')
  const [sortField, setSortField] = useState<IpSortField | null>(null)
  const [sortDirection, setSortDirection] = useState<SortDirection>('asc')
  const [requestStatusFilter, setRequestStatusFilter] = useState('Pending')
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState('')
  const [editTarget, setEditTarget] = useState<EditTarget | null>(null)
  const [draft, setDraft] = useState<IpDraft | null>(null)
  const [rejectTarget, setRejectTarget] = useState<IpRequestRecord | null>(null)
  const [rejectReason, setRejectReason] = useState('')
  const [isSaving, setIsSaving] = useState(false)
  const [isPinging, setIsPinging] = useState(false)
  const [pingingIp, setPingingIp] = useState<string | null>(null)
  const [pingSummary, setPingSummary] = useState<IpPingScanResult | null>(null)

  const loadData = useCallback(async () => {
    setIsLoading(true)
    setError('')
    try {
      if (!isTauri()) {
        setInventory(createMockInventory())
        const mockRequests = createMockRequests()
        setRequests(mockRequests)
        onPendingRequestCountChange(mockRequests.filter((item) => item.status === 'Pending').length)
        return
      }
      const [loadedInventory, loadedRequests] = await Promise.all([
        invoke<IpInventoryRecord[]>('get_ip_inventory'),
        invoke<IpRequestRecord[]>('get_ip_requests', { status: null }),
      ])
      setInventory(loadedInventory)
      setRequests(loadedRequests)
      onPendingRequestCountChange(loadedRequests.filter((item) => item.status === 'Pending').length)
      onActivity(`IP Management refreshed: ${loadedInventory.length} addresses`)
    } catch (loadError) {
      setError(formatError(loadError))
    } finally {
      setIsLoading(false)
    }
  }, [onActivity, onPendingRequestCountChange])

  useEffect(() => {
    const initialLoad = window.setTimeout(() => {
      void loadData()
    }, 0)
    return () => window.clearTimeout(initialLoad)
  }, [loadData])

  useEffect(() => {
    if (!isTauri()) return undefined

    let active = true
    let inFlight = false
    const refreshRequests = async () => {
      if (inFlight) return
      inFlight = true
      try {
        const loadedRequests = await invoke<IpRequestRecord[]>('get_ip_requests', { status: null })
        if (active) {
          setRequests(loadedRequests)
          onPendingRequestCountChange(loadedRequests.filter((item) => item.status === 'Pending').length)
        }
      } catch {
        // The manual refresh remains available if the central service is temporarily unavailable.
      } finally {
        inFlight = false
      }
    }

    const interval = window.setInterval(() => void refreshRequests(), 4_000)
    return () => {
      active = false
      window.clearInterval(interval)
    }
  }, [onPendingRequestCountChange])

  const metrics = useMemo(
    () => ({
      total: inventory.length,
      used: inventory.filter((item) => item.status === 'Used').length,
      unused: inventory.filter((item) => item.status === 'Unused').length,
      reserved: inventory.filter((item) => item.status === 'Reserved').length,
      pending: requests.filter((item) => item.status === 'Pending').length,
    }),
    [inventory, requests],
  )

  const visibleInventory = useMemo(() => {
    const needle = query.trim().toLowerCase()
    const recordsForStatus = statusFilter === 'all'
      ? inventory
      : inventory.filter((item) => item.status === statusFilter)
    const filtered = recordsForStatus.filter((item) => {
      if (!needle) return true
      return [
        item.ipAddress,
        item.status,
        item.location,
        item.responsible,
        item.environment,
        item.deviceType,
        item.serialNumber,
        item.antenna1,
        item.antenna2,
        item.usageEndDate,
        item.notes,
      ]
        .filter(Boolean)
        .join(' ')
        .toLowerCase()
        .includes(needle)
    })

    if (!sortField) return filtered

    return [...filtered].sort((left, right) => {
      const leftValue = inventorySortValue(left, sortField)
      const rightValue = inventorySortValue(right, sortField)
      if (!leftValue && !rightValue) return left.sortOrder - right.sortOrder
      if (!leftValue) return 1
      if (!rightValue) return -1
      const comparison = leftValue.localeCompare(rightValue, undefined, { numeric: true, sensitivity: 'base' })
      return comparison === 0
        ? left.sortOrder - right.sortOrder
        : sortDirection === 'asc' ? comparison : -comparison
    })
  }, [inventory, query, sortDirection, sortField, statusFilter])

  const changeSort = (field: IpSortField) => {
    if (sortField === field) {
      setSortDirection((current) => current === 'asc' ? 'desc' : 'asc')
      return
    }
    setSortField(field)
    setSortDirection('asc')
  }

  const sortHeader = (label: string, field: IpSortField) => {
    const isActive = sortField === field
    const SortIcon = !isActive ? ArrowUpDown : sortDirection === 'asc' ? ArrowUp : ArrowDown
    return (
      <th aria-sort={isActive ? (sortDirection === 'asc' ? 'ascending' : 'descending') : 'none'}>
        <button className="ip-sort-button" type="button" onClick={() => changeSort(field)}>
          <span>{label}</span>
          <SortIcon size={13} aria-hidden="true" />
        </button>
      </th>
    )
  }

  const runPingScan = async () => {
    if (isPinging) return
    setIsPinging(true)
    setError('')
    try {
      if (!isTauri()) {
        const completedAt = new Date().toISOString()
        const onlineCount = inventory.filter((item) => item.isOnline === true).length
        setPingSummary({
          totalCount: inventory.length,
          onlineCount,
          offlineCount: inventory.length - onlineCount,
          startedAt: completedAt,
          completedAt,
        })
      } else {
        const result = await invoke<IpPingScanResult>('run_ip_ping_scan')
        setPingSummary(result)
        await loadData()
        onActivity(`IP ping completed: ${result.onlineCount} online, ${result.offlineCount} offline`)
      }
    } catch (pingError) {
      setError(formatError(pingError))
    } finally {
      setIsPinging(false)
    }
  }

  const runSinglePing = async (item: IpInventoryRecord) => {
    if (isPinging || pingingIp) return
    setPingingIp(item.ipAddress)
    setError('')
    try {
      const result: IpPingResult = isTauri()
        ? await invoke<IpPingResult>('run_single_ip_ping', { ipAddress: item.ipAddress })
        : { ipAddress: item.ipAddress, isOnline: item.isOnline ?? false, checkedAt: new Date().toISOString() }
      setInventory((current) => current.map((record) => (
        record.ipAddress === result.ipAddress
          ? { ...record, isOnline: result.isOnline, lastPingAt: result.checkedAt }
          : record
      )))
      onActivity(`IP ping completed: ${result.ipAddress} is ${result.isOnline ? 'online' : 'offline'}`)
    } catch (pingError) {
      setError(formatError(pingError))
    } finally {
      setPingingIp(null)
    }
  }

  const visibleRequests = useMemo(() => {
    const needle = query.trim().toLowerCase()
    return requests.filter((request) => {
      if (requestStatusFilter !== 'all' && request.status !== requestStatusFilter) return false
      if (!needle) return true
      return [request.ipAddress, request.requestedBy, request.location, request.responsible,
        request.deviceType, request.antenna1, request.antenna2]
        .filter(Boolean)
        .join(' ')
        .toLowerCase()
        .includes(needle)
    })
  }, [query, requestStatusFilter, requests])

  const openInventoryEdit = (ip: IpInventoryRecord) => {
    setEditTarget({ kind: 'inventory', ip })
    setDraft(draftFromIp(ip))
  }

  const openRequestApproval = (request: IpRequestRecord) => {
    const ip = inventory.find((item) => item.ipAddress === request.ipAddress)
    if (!ip) {
      setError(`IP record ${request.ipAddress} was not found.`)
      return
    }
    setEditTarget({ kind: 'request', ip, request })
    setDraft(draftFromRequest(request))
  }

  const closeEdit = () => {
    if (isSaving) return
    setEditTarget(null)
    setDraft(null)
  }

  const saveEdit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!editTarget || !draft) return
    setIsSaving(true)
    setError('')
    try {
      if (!isTauri()) {
        const updated: IpInventoryRecord = {
          ...editTarget.ip,
          ...draft,
          location: draft.location || null,
          responsible: draft.responsible || null,
          environment: draft.environment || null,
          deviceType: draft.deviceType || null,
          serialNumber: draft.serialNumber || null,
          antenna1: draft.antenna1 || null,
          antenna2: draft.antenna2 || null,
          notes: draft.notes || null,
          usageEndDate: draft.usageEndDate || null,
          rowVersion: editTarget.ip.rowVersion + 1,
          updatedAt: new Date().toISOString(),
        }
        setInventory((current) => current.map((item) => (item.ipAddress === updated.ipAddress ? updated : item)))
        if (editTarget.kind === 'request') {
          setRequests((current) =>
            current.map((item) =>
              item.id === editTarget.request.id
                ? { ...item, status: 'Approved', rowVersion: item.rowVersion + 1, updatedAt: new Date().toISOString() }
                : item,
            ),
          )
        }
      } else if (editTarget.kind === 'inventory') {
        const updated = await invoke<IpInventoryRecord>('update_ip_inventory', {
          ipAddress: editTarget.ip.ipAddress,
          request: { ...draft, rowVersion: editTarget.ip.rowVersion },
        })
        setInventory((current) => current.map((item) => (item.ipAddress === updated.ipAddress ? updated : item)))
      } else {
        await invoke<IpRequestRecord>('approve_ip_request', {
          requestId: editTarget.request.id,
          request: {
            location: draft.location || null,
            responsible: draft.responsible || null,
            environment: draft.environment || null,
            deviceType: draft.deviceType || null,
            serialNumber: draft.serialNumber || null,
            antenna1: draft.antenna1 || null,
            antenna2: draft.antenna2 || null,
            notes: draft.notes || null,
            usageEndDate: draft.usageEndDate || null,
            requestRowVersion: editTarget.request.rowVersion,
            ipRowVersion: editTarget.ip.rowVersion,
          },
        })
        await loadData()
      }
      onActivity(
        editTarget.kind === 'request'
          ? `IP request approved for ${editTarget.ip.ipAddress}`
          : `IP record updated: ${editTarget.ip.ipAddress}`,
      )
      setEditTarget(null)
      setDraft(null)
    } catch (saveError) {
      setError(formatError(saveError))
    } finally {
      setIsSaving(false)
    }
  }

  const rejectRequest = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!rejectTarget || !rejectReason.trim()) return
    setIsSaving(true)
    setError('')
    try {
      if (isTauri()) {
        await invoke<IpRequestRecord>('reject_ip_request', {
          requestId: rejectTarget.id,
          request: { reason: rejectReason.trim(), rowVersion: rejectTarget.rowVersion },
        })
        await loadData()
      } else {
        setRequests((current) =>
          current.map((item) =>
            item.id === rejectTarget.id
              ? {
                  ...item,
                  status: 'Rejected',
                  decisionReason: rejectReason.trim(),
                  rowVersion: item.rowVersion + 1,
                }
              : item,
          ),
        )
      }
      onActivity(`IP request rejected for ${rejectTarget.ipAddress}`)
      setRejectTarget(null)
      setRejectReason('')
    } catch (rejectError) {
      setError(formatError(rejectError))
    } finally {
      setIsSaving(false)
    }
  }

  return (
    <section className="surface ip-management-surface">
      <div className="ip-management-header">
        <div className="ip-management-tabs" role="group" aria-label="IP Management sections">
          <button aria-pressed={activeTab === 'inventory'} className={activeTab === 'inventory' ? 'active' : ''} onClick={() => setActiveTab('inventory')}>
            IP Addresses
          </button>
          <button aria-pressed={activeTab === 'requests'} className={activeTab === 'requests' ? 'active' : ''} onClick={() => setActiveTab('requests')}>
            Requests
            {metrics.pending > 0 && <span>{metrics.pending}</span>}
          </button>
        </div>
        <div className="ip-management-actions">
          <button className="secondary-button" onClick={() => void runPingScan()} disabled={isLoading || isPinging || pingingIp !== null}>
            <Activity size={16} className={isPinging ? 'spin' : ''} />
            {isPinging ? 'Checking...' : 'Ping all'}
          </button>
          <button className="secondary-button" onClick={() => void loadData()} disabled={isLoading || isPinging || pingingIp !== null}>
            <RefreshCw size={16} className={isLoading ? 'spin' : ''} />
            Refresh
          </button>
        </div>
      </div>

      <div className="ip-management-metrics" aria-label="IP inventory summary">
        <div className="ip-metric-total"><span>Total</span><strong>{isLoading ? '—' : metrics.total}</strong></div>
        <div className="ip-metric-used"><span>Used</span><strong>{isLoading ? '—' : metrics.used}</strong></div>
        <div className="ip-metric-unused"><span>Unused</span><strong>{isLoading ? '—' : metrics.unused}</strong></div>
        <div className="ip-metric-reserved"><span>Reserved</span><strong>{isLoading ? '—' : metrics.reserved}</strong></div>
        <div className="ip-metric-pending"><span>Pending requests</span><strong>{isLoading ? '—' : metrics.pending}</strong></div>
      </div>

      <div className="ip-management-toolbar">
        <label className="ip-search-box">
          <Search size={17} />
          <input
            type="search"
            aria-label={activeTab === 'inventory' ? 'Search IP addresses' : 'Search requests'}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={activeTab === 'inventory' ? 'Search IP, device, SN or owner' : 'Search requests'}
          />
        </label>
        {activeTab === 'inventory' ? (
          <div className="ip-status-filters" role="group" aria-label="Filter by IP status">
            {([
              ['all', 'All', metrics.total],
              ['Used', 'Used', metrics.used],
              ['Unused', 'Unused', metrics.unused],
              ['Reserved', 'Reserved', metrics.reserved],
            ] as Array<[IpStatusFilter, string, number]>).map(([value, label, count]) => (
              <button
                type="button"
                className={statusFilter === value ? 'active' : ''}
                aria-pressed={statusFilter === value}
                onClick={() => setStatusFilter(value)}
                key={value}
              >
                {label} <span>{count}</span>
              </button>
            ))}
          </div>
        ) : (
          <select
            value={requestStatusFilter}
            onChange={(event) => setRequestStatusFilter(event.target.value)}
            aria-label="Request status"
          >
            <option value="all">All requests</option>
            <option value="Pending">Pending</option>
            <option value="Approved">Approved</option>
            <option value="Rejected">Rejected</option>
          </select>
        )}
        {(query || (activeTab === 'inventory' ? statusFilter !== 'all' : requestStatusFilter !== 'Pending')) && (
          <button className="ip-clear-filters" type="button" onClick={() => {
            setQuery('')
            if (activeTab === 'inventory') setStatusFilter('all')
            else setRequestStatusFilter('Pending')
          }}><X size={14} />Reset filters</button>
        )}
        <span className="ip-result-count" role="status">
          {isLoading ? 'Loading…' : activeTab === 'inventory'
            ? `${visibleInventory.length} of ${inventory.length} addresses`
            : `${visibleRequests.length} requests`}
        </span>
      </div>

      <div className="ip-management-notices">
        {activeTab === 'inventory' && pingSummary && (
          <div className="ip-ping-summary" role="status">
            <Activity size={16} />
            <span>Last ping: <strong>{pingSummary.onlineCount} online</strong>, {pingSummary.offlineCount} offline</span>
            <time dateTime={pingSummary.completedAt}>{formatDate(pingSummary.completedAt)}</time>
          </div>
        )}

        {error && (
          <div className="ip-management-error" role="alert">
            <AlertTriangle size={18} />
            <span>{error}</span>
            <button type="button" onClick={() => setError('')} aria-label="Close error"><X size={16} /></button>
          </div>
        )}
      </div>

      {activeTab === 'inventory' ? (
        <div className="ip-table-wrap" tabIndex={0} role="region" aria-label="IP addresses table" aria-busy={isLoading}>
          <table className="ip-table" aria-label="IP address inventory">
            <thead>
              <tr>
                <th>IP Address</th><th>Status</th><th>Online</th>
                {sortHeader('Location', 'location')}
                {sortHeader('Responsible', 'responsible')}
                <th>Environment</th>
                {sortHeader('Device', 'deviceType')}
                <th>SN</th>
                {sortHeader('Antennas', 'antennas')}
                {sortHeader('Usage period', 'usageEndDate')}
                <th>Notes</th><th aria-label="Actions" />
              </tr>
            </thead>
            <tbody>
              {visibleInventory.map((item) => {
                const conflict = item.status === 'Unused' && item.isOnline === true
                const usageExpired = isUsageExpired(item)
                return (
                  <tr className={[conflict ? 'ip-conflict-row' : '', usageExpired ? 'ip-usage-expired-row' : ''].filter(Boolean).join(' ')} key={item.ipAddress}>
                    <td className="ip-address-cell">
                      <strong>{item.ipAddress}</strong>
                      {conflict && <span title="Online address is marked as unused"><AlertTriangle size={13} /> Conflict</span>}
                    </td>
                    <td><span className={`ip-status-badge ${item.status.toLowerCase()}`}>{item.status}</span></td>
                    <td>
                      <span className={`ip-online-state ${item.isOnline === true ? 'online' : item.isOnline === false ? 'offline' : 'unknown'}`} title={`Last check: ${formatDate(item.lastPingAt)}`}>
                        <Circle size={9} fill="currentColor" /> {item.isOnline === true ? 'Online' : item.isOnline === false ? 'Offline' : 'Unknown'}
                      </span>
                    </td>
                    <td title={item.location || undefined}>{item.location || '--'}</td><td title={item.responsible || undefined}>{item.responsible || '--'}</td><td title={item.environment || undefined}>{item.environment || '--'}</td>
                    <td title={item.deviceType || undefined}>{item.deviceType || '--'}</td><td title={item.serialNumber || undefined}>{item.serialNumber || '--'}</td>
                    <td title={formatAntennas(item.antenna1, item.antenna2)}>{formatAntennas(item.antenna1, item.antenna2)}</td>
                    <td className={usageExpired ? 'ip-usage-expired-cell' : ''}>{formatUsagePeriod(item.usageEndDate)}</td>
                    <td className="ip-notes-cell" title={item.notes || ''}>{item.notes || '--'}</td>
                    <td>
                      <div className="ip-row-actions">
                        <button
                          className="ip-edit-button"
                          onClick={() => void runSinglePing(item)}
                          disabled={isPinging || pingingIp !== null}
                          title={`Ping ${item.ipAddress}`}
                          aria-label={`Ping ${item.ipAddress}`}
                        >
                          <Activity size={15} className={pingingIp === item.ipAddress ? 'spin' : ''} />
                        </button>
                        <button className="ip-edit-button" onClick={() => openInventoryEdit(item)} title={`Edit ${item.ipAddress}`} aria-label={`Edit ${item.ipAddress}`}><Pencil size={15} /></button>
                      </div>
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
          {isLoading && <div className="ip-loading-state" role="status"><RefreshCw size={20} className="spin" />Loading IP addresses…</div>}
          {!isLoading && visibleInventory.length === 0 && <div className="ip-empty-state"><Search size={24} /><strong>No matching addresses</strong><span>Try another search or reset the filters.</span></div>}
        </div>
      ) : (
        <div className="ip-request-list" aria-busy={isLoading}>
          {visibleRequests.map((request) => (
            <article className="ip-request-row" key={request.id}>
              <div className="ip-request-main">
                <div><strong>{request.ipAddress}</strong><span className={`ip-status-badge ${request.status.toLowerCase()}`}>{request.status}</span></div>
                <p>{request.requestedBy}</p>
              </div>
              <div className="ip-request-field"><span>Location</span><strong>{request.location || '--'}</strong></div>
              <div className="ip-request-field"><span>Responsible</span><strong>{request.responsible || '--'}</strong></div>
              <div className="ip-request-field"><span>Environment</span><strong>{request.environment || '--'}</strong></div>
              <div className="ip-request-field"><span>Device</span><strong>{request.deviceType || '--'}</strong></div>
              <div className="ip-request-field"><span>Antennas</span><strong>{formatAntennas(request.antenna1, request.antenna2)}</strong></div>
              <div className="ip-request-field"><span>Usage period</span><strong>{formatUsagePeriod(request.usageEndDate)}</strong></div>
              <div className="ip-request-field request-date"><span>Created</span><strong>{formatDate(request.createdAt)}</strong></div>
              <div className="ip-request-actions">
                {request.status === 'Pending' ? (
                  <>
                    <button className="secondary-button" onClick={() => { setRejectTarget(request); setRejectReason('') }}><X size={15} />Reject</button>
                    <button className="primary-button" onClick={() => openRequestApproval(request)}><Check size={15} />Approve</button>
                  </>
                ) : (
                  <span className="ip-request-decision"><Clock3 size={14} />{request.decisionBy || 'Completed'}</span>
                )}
              </div>
            </article>
          ))}
          {isLoading && <div className="ip-loading-state" role="status"><RefreshCw size={20} className="spin" />Loading requests…</div>}
          {!isLoading && visibleRequests.length === 0 && <div className="ip-empty-state"><Check size={24} /><strong>{requestStatusFilter === 'Pending' && !query ? 'No pending requests' : 'No matching requests'}</strong><span>{requestStatusFilter === 'Pending' && !query ? 'New requests will appear here automatically.' : 'Try another search or request status.'}</span></div>}
        </div>
      )}

      {editTarget && draft && (
        <div className="modal-layer">
          <button className="modal-backdrop" type="button" onClick={closeEdit} aria-label="Close dialog" />
          <section className="ip-edit-modal" role="dialog" aria-modal="true" aria-labelledby="ip-edit-title">
            <header>
              <div><span>{editTarget.kind === 'request' ? 'Approve request' : 'Edit IP address'}</span><h2 id="ip-edit-title">{editTarget.ip.ipAddress}</h2></div>
              <button className="modal-close" type="button" onClick={closeEdit} aria-label="Close"><X size={18} /></button>
            </header>
            <form onSubmit={saveEdit}>
              <div className="ip-edit-grid">
                <label><span>Status</span><select value={draft.status} disabled={editTarget.kind === 'request'} onChange={(event) => setDraft({ ...draft, status: event.target.value as IpStatus })}><option>Unused</option><option>Used</option><option>Reserved</option></select></label>
                <label><span>Location</span><input list="ip-location-options" value={draft.location} onChange={(event) => setDraft({ ...draft, location: event.target.value })} /></label>
                <label><span>Responsible</span><input value={draft.responsible} onChange={(event) => setDraft({ ...draft, responsible: event.target.value })} /></label>
                <label><span>Environment</span><select value={draft.environment} onChange={(event) => setDraft({ ...draft, environment: event.target.value })}>{ENVIRONMENT_OPTIONS.map((option) => <option value={option} key={option || 'empty'}>{option || 'Not set'}</option>)}</select></label>
                <label><span>Device / Type</span><input list="ip-device-options" value={draft.deviceType} onChange={(event) => setDraft({ ...draft, deviceType: event.target.value })} /></label>
                <label><span>Serial number</span><input value={draft.serialNumber} onChange={(event) => setDraft({ ...draft, serialNumber: event.target.value })} /></label>
                <label><span>Antenna 1</span><input maxLength={120} value={draft.antenna1} onChange={(event) => setDraft({ ...draft, antenna1: event.target.value })} /></label>
                <label><span>Antenna 2</span><input maxLength={120} value={draft.antenna2} onChange={(event) => setDraft({ ...draft, antenna2: event.target.value })} /></label>
                <label>
                  <span>Usage period</span>
                  <select value={draft.usageEndDate ? 'date' : 'none'} onChange={(event) => setDraft({ ...draft, usageEndDate: event.target.value === 'date' ? todayValue() : '' })}>
                    <option value="none">No end date</option>
                    <option value="date">Select end date</option>
                  </select>
                </label>
                {draft.usageEndDate && <label><span>End date</span><input type="date" value={draft.usageEndDate} onChange={(event) => setDraft({ ...draft, usageEndDate: event.target.value })} required /></label>}
                <label className="ip-edit-notes"><span>Notes</span><textarea rows={3} value={draft.notes} onChange={(event) => setDraft({ ...draft, notes: event.target.value })} /></label>
              </div>
              <datalist id="ip-location-options">{LOCATION_OPTIONS.map((option) => <option value={option} key={option} />)}</datalist>
              <datalist id="ip-device-options">{DEVICE_OPTIONS.map((option) => <option value={option} key={option} />)}</datalist>
              <footer><button className="secondary-button" type="button" onClick={closeEdit}>Cancel</button><button className="primary-button" type="submit" disabled={isSaving}>{isSaving ? 'Saving...' : editTarget.kind === 'request' ? 'Approve request' : 'Save changes'}</button></footer>
            </form>
          </section>
        </div>
      )}

      {rejectTarget && (
        <div className="modal-layer">
          <button className="modal-backdrop" type="button" onClick={() => setRejectTarget(null)} aria-label="Close dialog" />
          <section className="ip-reject-modal" role="dialog" aria-modal="true" aria-labelledby="ip-reject-title">
            <header><div><span>Reject request</span><h2 id="ip-reject-title">{rejectTarget.ipAddress}</h2></div><button className="modal-close" type="button" onClick={() => setRejectTarget(null)} aria-label="Close"><X size={18} /></button></header>
            <form onSubmit={rejectRequest}><label><span>Reason</span><textarea rows={4} required value={rejectReason} onChange={(event) => setRejectReason(event.target.value)} /></label><footer><button className="secondary-button" type="button" onClick={() => setRejectTarget(null)}>Cancel</button><button className="danger-button" type="submit" disabled={isSaving || !rejectReason.trim()}>Reject request</button></footer></form>
          </section>
        </div>
      )}
    </section>
  )
}
