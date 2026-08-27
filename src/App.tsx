import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type DragEvent,
  type FormEvent,
  type ReactNode,
} from 'react'
import { createPortal } from 'react-dom'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow, type DragDropEvent } from '@tauri-apps/api/window'
import { ModusWcNavbar, ModusWcThemeSwitcher } from '@trimble-oss/moduswebcomponents-react'
import qaCenterIcon from './assets/qa_center_icon.png'
import {
  Activity,
  AlertTriangle,
  BarChart3,
  CalendarDays,
  ChevronLeft,
  ChevronRight,
  Clock3,
  ClipboardList,
  Copy,
  Cpu,
  ExternalLink,
  Eye,
  EyeOff,
  FileText,
  Gauge,
  History,
  KeyRound,
  LayoutDashboard,
  Loader2,
  Lock,
  Minimize2,
  MoreHorizontal,
  Network,
  Pencil,
  Plus,
  Radio,
  RefreshCw,
  Save,
  Search,
  Settings,
  ShieldCheck,
  Server,
  Trash2,
  X,
  UploadCloud,
  UserRound,
} from 'lucide-react'
import './App.css'

type FirmwarePage =
  | 'FirmwareAlpha'
  | 'FirmwareBeta'
  | 'FirmwareCreateGroup'
  | 'FirmwareSingle'
  | 'FirmwareMultiple'
type SettingsPage = 'SettingsUpdateSafety'
type Page = 'Dashboard' | 'Receivers' | FirmwarePage | SettingsPage | 'Diagnostics' | 'Reports' | 'History'
type ReceiverStatus = 'online' | 'offline' | 'pending'

type Receiver = {
  ip: string
  group: number
  status: ReceiverStatus
  receiverName: string | null
  firmwareVersion: string | null
  firmwareDate: string | null
  rtkVersion: string | null
  testMode: boolean | null
  uptimeSeconds: number | null
  celsius: number | null
  svSolved: number | null
  fixType: string | null
  errorWarning: number | null
  needsReview: boolean
  lastError: string | null
}

type ReceiverLoginDetails = {
  ip: string
  user: string
  password: string
}

type FirmwareReceiver = {
  ip: string
  model: string
  group: number
  groupName: string
  hasCredentials: boolean
}

type FirmwareConfigResponse = {
  mode: string
  group: number
  groupName: string
  firmwareVersion: string
  defaultFirmwareSuffix: string
  logo: string
  receivers: FirmwareReceiver[]
}

type FirmwareReleaseCatalog = {
  sourceUrl: string
  refreshedAt: string
  releases: Array<{
    id: string
    name: string
    url: string
  }>
}

type FirmwareBuildCatalog = {
  releaseId: string
  releaseName: string
  sourceUrl: string
  refreshedAt: string
  builds: Array<{
    id: string
    name: string
    url: string
  }>
}

type StandaloneFirmwareBuild = FirmwareBuildCatalog['builds'][number] & {
  mode: 'Alpha' | 'Beta'
}

type FirmwareReleaseInspection = {
  releaseId: string
  releaseName: string
  buildId: string
  buildName: string
  sourceUrl: string
  refreshedAt: string
  packages: Array<{
    fileName: string
    url: string
  }>
  modelMatches: Array<{
    model: string
    status: 'Matched' | 'Missing' | 'Ambiguous'
    selectedFile: string | null
    candidates: string[]
  }>
}

type FirmwarePlanRequest = {
  mode: string
  firmwareVersion: string
  firmwareSuffix: string
  receiverIps: string[]
  parallel: boolean
  packageOverrides?: Record<string, string>
  localPackagePaths?: Record<string, string>
}

type FirmwareJobStatus = {
  jobId: string
  mode: string
  group: number
  groupName: string
  firmwareVersion: string
  firmwareSuffix: string
  parallel: boolean
  status: string
  progress: number
  createdAt: string
  completedAt: string | null
  receivers: Array<{
    ip: string
    model: string
    targetFile: string
    status: string
    phase: string
    progress: number
    currentFirmware: string | null
    message: string
    downgradeRequired: boolean
    downgradeDecision: boolean | null
    childJobId: string
  }>
  log: string[]
}

type FirmwareSingleUpdateJobStatus = {
  jobId: string
  batchId: string | null
  ip: string
  model: string
  targetFile: string
  status: string
  phase: string
  message: string
  receiverName: string | null
  firmwareVersion: string | null
  uptimeSeconds: number | null
  firmwareStatus: string | null
  fwStatus: string | null
  updateMode: string | null
  updatePercent: number | null
  updateStage: number | null
  updateStagePercent: number | null
  updateInProgress: boolean
  downgradeRequired: boolean
  downgradeDecision: boolean | null
  activeChecksum: string | null
  lastSeenAt: string | null
  cloneFileName: string | null
  cloneFilePath: string | null
  startedAt: string
  completedAt: string | null
  steps: Array<{
    state: string
    status: string
    message: string
  }>
}

type FirmwareSingleUpdateHistoryEntry = {
  jobId: string
  batchId?: string | null
  operationType?: string | null
  groupName?: string | null
  ip: string
  receiverName: string | null
  model: string
  targetFile: string
  startedAt: string
  completedAt: string | null
  result: string
  finalFirmwareVersion: string | null
  cloneFileName: string | null
  cloneFilePath: string | null
  errors: string[]
}

type MultipleUpdateItem = {
  ip: string
  receiverName: string | null
  model: string
  status: 'Queued' | 'Starting' | 'Running' | 'Completed' | 'Failed'
  progress: number
  phase: string
  message: string
  currentFirmware: string | null
  targetFile: string
  jobId: string | null
}

type MultipleUpdateBatch = {
  batchId: string
  status: 'Running' | 'Completed' | 'Failed'
  sourceMode: 'release' | 'local'
  firmwareVersion: string
  targetLabel: string
  startedAt: string
  items: MultipleUpdateItem[]
}

type PasswordRestoreResult = {
  ip: string
  status: string
  message: string
  restoredAt: string
}

type FirmwarePasswordRecoveryBatchResult = {
  jobId: string
  status: string
  totalCount: number
  restoredCount: number
  alreadyRestoredCount: number
  deferredCount: number
  failedCount: number
  receivers: PasswordRestoreResult[]
}

type CloneFileDownloadResult = {
  fileName: string
  destinationPath: string
}

type SelectedFirmwareFile = {
  name: string
  size: number
  path: string
}

type LocalFirmwarePackage = {
  name: string
  path: string
  size: number
}

type UpdateSafetySettings = {
  postRestartDelayMinutes: number
  preRestartSafetyMarginMinutes: number
  restartCycleMinutes: number
}

const navItems: Array<{ page: Page; label: string; icon: typeof LayoutDashboard }> = [
  { page: 'Dashboard', label: 'Dashboard', icon: LayoutDashboard },
  { page: 'Receivers', label: 'Receivers', icon: Radio },
  { page: 'Diagnostics', label: 'Diagnostics', icon: Activity },
  { page: 'Reports', label: 'Reports', icon: FileText },
  { page: 'History', label: 'History', icon: History },
]

const pageTitles: Record<Page, string> = {
  Dashboard: 'Dashboard',
  Receivers: 'Receivers',
  FirmwareAlpha: 'Alpha Receivers',
  FirmwareBeta: 'Beta Receivers',
  FirmwareCreateGroup: 'Create New Group',
  FirmwareSingle: 'Single Receiver Update',
  FirmwareMultiple: 'Multiple Receivers Update',
  Diagnostics: 'Diagnostics',
  Reports: 'Reports',
  History: 'History',
  SettingsUpdateSafety: 'Update Safety',
}

const firmwarePages: FirmwarePage[] = [
  'FirmwareAlpha',
  'FirmwareBeta',
  'FirmwareCreateGroup',
  'FirmwareSingle',
  'FirmwareMultiple',
]
const settingsPages: SettingsPage[] = ['SettingsUpdateSafety']

const statusLabels: Record<ReceiverStatus, string> = {
  online: 'Online',
  offline: 'Offline',
  pending: 'Pending',
}

const ACTION_MENU_WIDTH = 238
const ACTION_MENU_HEIGHT = 186
const ACTION_MENU_GAP = 8
const VIEWPORT_GUTTER = 12
const LOGIN_ENABLED = false
const RECEIVER_MODEL_OPTIONS = [
  'argon',
  'alloy',
  'bcuda',
  'krypton',
  'koolaid',
  'silk',
  'stella',
  'tungsten',
  'yeager',
]

function firmwareModelFromPackageName(fileName: string) {
  const tokens = fileName
    .toLowerCase()
    .replace(/\.(?:timg|img)$/i, '')
    .split(/[^a-z0-9]+/)
    .filter(Boolean)
  return RECEIVER_MODEL_OPTIONS.find((model) => tokens.includes(model)) || ''
}

type UpdateReceiverSummaryProps = {
  name: string
  ip: string
  groupLabel: string
  model: string
  status: ReceiverStatus
  firmwareVersion: string | null
  uptimeSeconds: number | null
  fixType?: string | null
  celsius?: number | null
  svSolved?: number | null
  credentialsReady?: boolean
  footer?: ReactNode
}

function UpdateReceiverSummary({
  name,
  ip,
  groupLabel,
  model,
  status,
  firmwareVersion,
  uptimeSeconds,
  fixType,
  celsius,
  svSolved,
  credentialsReady,
  footer,
}: UpdateReceiverSummaryProps) {
  const normalizedModel = model.trim()
  const modelUnknown = !normalizedModel || normalizedModel.toLowerCase() === 'unknown'

  return (
    <div className="update-receiver-summary">
      <div className="update-receiver-heading">
        <strong title={name}>{name}</strong>
        <span className={`model-pill ${modelUnknown ? 'unknown' : ''}`}>
          {modelUnknown ? 'Unknown' : normalizedModel}
        </span>
        {credentialsReady !== undefined ? (
          <span className={`credential-pill ${credentialsReady ? '' : 'missing'}`}>
            {credentialsReady ? 'Credentials OK' : 'Missing credentials'}
          </span>
        ) : null}
        <span className={`status-pill ${status}`}>{statusLabels[status]}</span>
      </div>

      <span className="update-receiver-meta" title={`${ip} - ${groupLabel}`}>
        {ip} - {groupLabel}
      </span>

      <div className="update-receiver-stats">
        <span className="firmware-mini-stat" title={`Current firmware: ${firmwareVersion || 'unknown'}`}>
          <Gauge size={13} /> FW {firmwareVersion || '--'}
        </span>
        <span className="firmware-mini-stat" title={`Uptime: ${formatDuration(uptimeSeconds)}`}>
          <Activity size={13} /> Uptime {formatDuration(uptimeSeconds)}
        </span>
        {fixType !== undefined ? (
          <span className="firmware-mini-stat" title={`Fix type: ${fixType || 'unknown'}`}>
            <Radio size={13} /> {fixType || '--'}
          </span>
        ) : null}
        {celsius !== undefined ? (
          <span className="firmware-mini-stat" title={`Temperature: ${formatNumber(celsius, 'C')}`}>
            <Cpu size={13} /> {formatNumber(celsius, 'C')}
          </span>
        ) : null}
        {svSolved !== undefined ? (
          <span className="firmware-mini-stat" title={`Solved satellites: ${formatNumber(svSolved)}`}>
            <BarChart3 size={13} /> SV {formatNumber(svSolved)}
          </span>
        ) : null}
      </div>

      {footer}
    </div>
  )
}

type FirmwareDeviceCardProps = UpdateReceiverSummaryProps & {
  selected?: boolean
  leading?: ReactNode
  actions?: ReactNode
  onSelect?: () => void
  ariaLabel?: string
  className?: string
}

function FirmwareDeviceCard({
  name,
  ip,
  groupLabel,
  model,
  status,
  firmwareVersion,
  uptimeSeconds,
  fixType,
  celsius,
  svSolved,
  credentialsReady,
  footer,
  selected = false,
  leading,
  actions,
  onSelect,
  ariaLabel,
  className = '',
}: FirmwareDeviceCardProps) {
  const normalizedModel = model.trim()
  const modelUnknown = !normalizedModel || normalizedModel.toLowerCase() === 'unknown'
  const cardClassName = [
    'firmware-device-card',
    status,
    selected ? 'selected' : '',
    onSelect ? 'interactive' : '',
    className,
  ]
    .filter(Boolean)
    .join(' ')

  const content = (
    <>
      {leading ? <div className="firmware-device-card__leading">{leading}</div> : null}
      <div className="firmware-device-card__summary">
        <div className="firmware-device-card__heading">
          <strong title={name}>{name}</strong>
          <span className={`firmware-device-card__model ${modelUnknown ? 'unknown' : ''}`}>
            {modelUnknown ? 'Unknown' : normalizedModel}
          </span>
          {credentialsReady !== undefined ? (
            <span className={`firmware-device-card__credentials ${credentialsReady ? '' : 'missing'}`}>
              {credentialsReady ? 'Credentials OK' : 'Missing credentials'}
            </span>
          ) : null}
          <span className={`firmware-device-card__status ${status}`}>{statusLabels[status]}</span>
        </div>

        <span className="firmware-device-card__meta" title={`${ip} - ${groupLabel}`}>
          {ip} - {groupLabel}
        </span>

        <div className="firmware-device-card__stats">
          <span title={`Current firmware: ${firmwareVersion || 'unknown'}`}>
            <Gauge size={13} /> FW {firmwareVersion || '--'}
          </span>
          <span title={`Uptime: ${formatDuration(uptimeSeconds)}`}>
            <Activity size={13} /> Uptime {formatDuration(uptimeSeconds)}
          </span>
          {fixType !== undefined ? (
            <span title={`Fix type: ${fixType || 'unknown'}`}>
              <Radio size={13} /> {fixType || '--'}
            </span>
          ) : null}
          {celsius !== undefined ? (
            <span title={`Temperature: ${formatNumber(celsius, 'C')}`}>
              <Cpu size={13} /> {formatNumber(celsius, 'C')}
            </span>
          ) : null}
          {svSolved !== undefined ? (
            <span title={`Solved satellites: ${formatNumber(svSolved)}`}>
              <BarChart3 size={13} /> SV {formatNumber(svSolved)}
            </span>
          ) : null}
        </div>

        {footer ? <div className="firmware-device-card__footer">{footer}</div> : null}
      </div>
      {actions ? <div className="firmware-device-card__actions">{actions}</div> : null}
    </>
  )

  if (onSelect) {
    return (
      <button className={cardClassName} type="button" onClick={onSelect} aria-label={ariaLabel}>
        {content}
      </button>
    )
  }

  return <article className={cardClassName}>{content}</article>
}

type ActionMenuPosition = {
  top: number
  left: number
}

function App() {
  const [isAuthenticated, setIsAuthenticated] = useState(!LOGIN_ENABLED)
  const [isInitialLoading, setIsInitialLoading] = useState(false)
  const [loginUser, setLoginUser] = useState('')
  const [loginPassword, setLoginPassword] = useState('')
  const [showPassword, setShowPassword] = useState(false)
  const [loginError, setLoginError] = useState('')
  const [loginDetailsTarget, setLoginDetailsTarget] = useState<Receiver | null>(null)
  const [loginDetailsPassword, setLoginDetailsPassword] = useState('')
  const [loginDetails, setLoginDetails] = useState<ReceiverLoginDetails | null>(null)
  const [loginDetailsError, setLoginDetailsError] = useState('')
  const [isLoginDetailsLoading, setIsLoginDetailsLoading] = useState(false)
  const [showLoginDetailsPassword, setShowLoginDetailsPassword] = useState(false)
  const [showReceiverPassword, setShowReceiverPassword] = useState(false)
  const [activePage, setActivePage] = useState<Page>('Dashboard')
  const [lastFirmwarePage, setLastFirmwarePage] = useState<FirmwarePage>('FirmwareSingle')
  const [isFirmwareOpen, setIsFirmwareOpen] = useState(false)
  const [isFirmwareGroupOpen, setIsFirmwareGroupOpen] = useState(false)
  const [isSettingsOpen, setIsSettingsOpen] = useState(false)
  const [receivers, setReceivers] = useState<Receiver[]>([])
  const [query, setQuery] = useState('')
  const [receiverFwFilter, setReceiverFwFilter] = useState('all')
  const [receiverFixFilter, setReceiverFixFilter] = useState('all')
  const [receiverStatusFilter, setReceiverStatusFilter] = useState('all')
  const isScanningRef = useRef(false)
  const [logLines, setLogLines] = useState<string[]>(() => {
    const stamp = new Date().toLocaleTimeString('en-GB', { hour12: false })
    return [`[${stamp}] QA Receiver Center started`, `[${stamp}] Receiver list source: QA network config`]
  })

  const addLog = useCallback((message: string) => {
    const stamp = new Date().toLocaleTimeString('en-GB', { hour12: false })
    setLogLines((current) => [`[${stamp}] ${message}`, ...current].slice(0, 40))
  }, [])

  const navigateToPage = useCallback((page: Page) => {
    setActivePage(page)
    if (firmwarePages.includes(page as FirmwarePage)) {
      setLastFirmwarePage(page as FirmwarePage)
    }
  }, [])

  const scanReceivers = useCallback(async (silent = false) => {
    if (isScanningRef.current) {
      return false
    }

    try {
      isScanningRef.current = true
      if (!silent) {
        addLog('Starting diagnostics scan')
      }
      const scanned = await invoke<Receiver[]>('scan_receivers')
      setReceivers(scanned)
      addLog(
        silent
          ? `Receiver list refreshed: ${scanned.length} receivers`
          : `Diagnostics finished: ${scanned.length} receivers scanned`,
      )
      return true
    } catch (error) {
      addLog(`Scan failed: ${formatError(error)}`)
      return false
    } finally {
      isScanningRef.current = false
    }
  }, [addLog])

  const handleLogin = useCallback(
    async (event: FormEvent<HTMLFormElement>) => {
      event.preventDefault()

      try {
        await invoke('authenticate_operator', {
          username: loginUser.trim(),
          password: loginPassword,
        })
      } catch {
        setLoginError('Invalid username or password.')
        return
      }

      setLoginError('')
      setIsAuthenticated(true)
      setIsInitialLoading(true)
      addLog(`Signed in as ${loginUser.trim()}`)
      await scanReceivers()
      setIsInitialLoading(false)
    },
    [addLog, loginPassword, loginUser, scanReceivers],
  )

  const openReceiver = useCallback(
    async (ip: string) => {
      try {
        await invoke('open_receiver', { ip })
        addLog(`Opened receiver page ${ip}`)
      } catch (error) {
        addLog(`Open failed: ${formatError(error)}`)
      }
    },
    [addLog],
  )

  const openLoginDetails = useCallback((receiver: Receiver) => {
    setLoginDetailsTarget(receiver)
    setLoginDetailsPassword('')
    setLoginDetails(null)
    setLoginDetailsError('')
    setShowLoginDetailsPassword(false)
    setShowReceiverPassword(false)
  }, [])

  const closeLoginDetails = useCallback(() => {
    setLoginDetailsTarget(null)
    setLoginDetailsPassword('')
    setLoginDetails(null)
    setLoginDetailsError('')
    setIsLoginDetailsLoading(false)
    setShowLoginDetailsPassword(false)
    setShowReceiverPassword(false)
  }, [])

  const loadLoginDetails = useCallback(
    async (event: FormEvent<HTMLFormElement>) => {
      event.preventDefault()
      if (!loginDetailsTarget) {
        return
      }

      try {
        setIsLoginDetailsLoading(true)
        setLoginDetailsError('')
        const details = await invoke<ReceiverLoginDetails>('get_receiver_login_details', {
          ip: loginDetailsTarget.ip,
          appPassword: loginDetailsPassword,
        })
        setLoginDetails(details)
        setShowReceiverPassword(false)
      } catch (error) {
        setLoginDetails(null)
        setLoginDetailsError(formatError(error))
      } finally {
        setIsLoginDetailsLoading(false)
      }
    },
    [loginDetailsPassword, loginDetailsTarget],
  )

  useEffect(() => {
    if (!isAuthenticated || isInitialLoading || receivers.length > 0) {
      return undefined
    }

    const startupScan = window.setTimeout(() => {
      void scanReceivers(true)
    }, 0)

    return () => {
      window.clearTimeout(startupScan)
    }
  }, [isAuthenticated, isInitialLoading, receivers.length, scanReceivers])

  useEffect(() => {
    if (!isAuthenticated || isInitialLoading) {
      return undefined
    }

    const interval = window.setInterval(() => {
      void scanReceivers(true)
    }, 120_000)

    return () => {
      window.clearInterval(interval)
    }
  }, [isAuthenticated, isInitialLoading, scanReceivers])

  const onlineCount = receivers.filter((receiver) => receiver.status === 'online').length
  const reviewCount = receivers.filter((receiver) => receiver.needsReview).length
  const offlineCount = receivers.filter((receiver) => receiver.status === 'offline').length
  const pendingCount = receivers.filter((receiver) => receiver.status === 'pending').length
  const offlineReceivers = receivers.filter((receiver) => receiver.status === 'offline')

  const visibleReceivers = useMemo(() => {
    const source = activePage === 'Dashboard' ? offlineReceivers : receivers
    const needle = activePage === 'Dashboard' ? '' : query.trim().toLowerCase()

    return source.filter((receiver) => {
      if (
        activePage === 'Receivers' &&
        receiverFwFilter !== 'all' &&
        (receiver.firmwareVersion || '--') !== receiverFwFilter
      ) {
        return false
      }

      if (
        activePage === 'Receivers' &&
        receiverFixFilter !== 'all' &&
        (receiver.fixType || '--') !== receiverFixFilter
      ) {
        return false
      }

      if (
        activePage === 'Receivers' &&
        receiverStatusFilter !== 'all' &&
        receiver.status !== receiverStatusFilter
      ) {
        return false
      }

      const haystack = [
        receiver.ip,
        receiver.group,
        receiver.receiverName,
        receiver.firmwareVersion,
        receiver.rtkVersion,
        receiver.fixType,
        receiver.lastError,
      ]
        .filter(Boolean)
        .join(' ')
        .toLowerCase()

      return !needle || haystack.includes(needle)
    })
  }, [activePage, offlineReceivers, query, receiverFixFilter, receiverFwFilter, receiverStatusFilter, receivers])

  const receiverFirmwareFilterOptions = useMemo(
    () =>
      Array.from(new Set(receivers.map((receiver) => receiver.firmwareVersion || '--')))
        .sort((a, b) => a.localeCompare(b, undefined, { numeric: true })),
    [receivers],
  )
  const receiverFixFilterOptions = useMemo(
    () =>
      Array.from(new Set(receivers.map((receiver) => receiver.fixType || '--')))
        .sort((a, b) => a.localeCompare(b, undefined, { numeric: true })),
    [receivers],
  )

  const isFirmwareActive = firmwarePages.includes(activePage as FirmwarePage)
  const isSettingsActive = settingsPages.includes(activePage as SettingsPage)
  const isFirmwareGroupActive =
    activePage === 'FirmwareAlpha' || activePage === 'FirmwareBeta' || activePage === 'FirmwareCreateGroup'
  const pageTitle = pageTitles[activePage]
  const pageEyebrow = isFirmwareActive
    ? 'Firmware operations'
    : isSettingsActive
      ? 'Application settings'
      : 'Receiver operations'
  const receiverSectionTitle = activePage === 'Dashboard' ? 'Requires Attention' : 'Receivers'
  const receiverSectionSubtitle =
    activePage === 'Dashboard'
      ? `${offlineReceivers.length} receivers offline`
      : `${receivers.length} loaded from qa-receivers.json`

  if (!isAuthenticated) {
    return (
      <LoginScreen
        username={loginUser}
        password={loginPassword}
        showPassword={showPassword}
        error={loginError}
        onUsernameChange={setLoginUser}
        onPasswordChange={setLoginPassword}
        onTogglePassword={() => setShowPassword((current) => !current)}
        onSubmit={handleLogin}
      />
    )
  }

  if (isInitialLoading) {
    return <BootLoadingScreen />
  }

  return (
    <main className="app-shell">
      <ModusWcNavbar
        className="app-navbar"
        logoName="trimble"
        userCard={{
          email: '',
          name: '',
        }}
        visibility={{ logo: true, user: false }}
      >
        <div className="navbar-product" slot="center" aria-label="QA Receiver Center">
          <img src={qaCenterIcon} alt="" aria-hidden="true" />
          <strong>QA Receiver Center</strong>
        </div>
        <div className="navbar-theme" slot="end">
          <span>Appearance</span>
          <ModusWcThemeSwitcher />
        </div>
      </ModusWcNavbar>

      <aside className="sidebar">
        <nav className="nav-list" aria-label="Primary navigation">
          {navItems.slice(0, 2).map((item) => (
            <button
              className={`nav-item ${activePage === item.page ? 'active' : ''}`}
              key={item.page}
              aria-label={item.label}
              title={item.label}
              onClick={() => navigateToPage(item.page)}
            >
              <item.icon size={18} />
              <span>{item.label}</span>
            </button>
          ))}

          <div className="nav-group">
            <button
              className={`nav-item nav-parent ${isFirmwareActive ? 'active' : ''}`}
              aria-label="Firmware"
              aria-expanded={isFirmwareOpen}
              title="Firmware"
              onClick={() => setIsFirmwareOpen((current) => !current)}
            >
              <UploadCloud size={18} />
              <span>Firmware</span>
            </button>

            {isFirmwareOpen && (
              <div className="nav-subtree">
                <button
                  className={`nav-subitem nav-parent ${isFirmwareGroupActive ? 'active' : ''}`}
                  aria-label="Group Update"
                  aria-expanded={isFirmwareGroupOpen}
                  title="Group Update"
                  onClick={() => setIsFirmwareGroupOpen((current) => !current)}
                >
                  <UploadCloud size={15} />
                  <span>Group Update</span>
                </button>

                {isFirmwareGroupOpen && (
                  <div className="nav-subtree nested">
                    <button
                      className={`nav-subitem ${activePage === 'FirmwareAlpha' ? 'active' : ''}`}
                      aria-label="Alpha Receivers"
                      title="Alpha Receivers"
                      onClick={() => navigateToPage('FirmwareAlpha')}
                    >
                      <Radio size={15} />
                      <span>Alpha Receivers</span>
                    </button>
                    <button
                      className={`nav-subitem ${activePage === 'FirmwareBeta' ? 'active' : ''}`}
                      aria-label="Beta Receivers"
                      title="Beta Receivers"
                      onClick={() => navigateToPage('FirmwareBeta')}
                    >
                      <Radio size={15} />
                      <span>Beta Receivers</span>
                    </button>
                    <button
                      className={`nav-subitem ${activePage === 'FirmwareCreateGroup' ? 'active' : ''}`}
                      aria-label="Create New Group"
                      title="Create New Group"
                      onClick={() => navigateToPage('FirmwareCreateGroup')}
                    >
                      <Plus size={15} />
                      <span>Create New Group</span>
                    </button>
                  </div>
                )}

                <button
                  className={`nav-subitem ${activePage === 'FirmwareSingle' ? 'active' : ''}`}
                  aria-label="Single Receiver Update"
                  title="Single Receiver Update"
                  onClick={() => navigateToPage('FirmwareSingle')}
                >
                  <UploadCloud size={15} />
                  <span>Single Receiver Update</span>
                </button>
                <button
                  className={`nav-subitem ${activePage === 'FirmwareMultiple' ? 'active' : ''}`}
                  aria-label="Multiple Receivers Update"
                  title="Multiple Receivers Update"
                  onClick={() => navigateToPage('FirmwareMultiple')}
                >
                  <UploadCloud size={15} />
                  <span>Multiple Receivers Update</span>
                </button>
              </div>
            )}
          </div>

          {navItems.slice(2).map((item) => (
            <button
              className={`nav-item ${activePage === item.page ? 'active' : ''}`}
              key={item.page}
              aria-label={item.label}
              title={item.label}
              onClick={() => navigateToPage(item.page)}
            >
              <item.icon size={18} />
              <span>{item.label}</span>
            </button>
          ))}

          <div className="nav-group">
            <button
              className={`nav-item nav-parent ${isSettingsActive ? 'active' : ''}`}
              aria-label="Settings"
              aria-expanded={isSettingsOpen}
              title="Settings"
              onClick={() => setIsSettingsOpen((current) => !current)}
            >
              <Settings size={18} />
              <span>Settings</span>
            </button>

            {isSettingsOpen && (
              <div className="nav-subtree">
                <button
                  className={`nav-subitem ${activePage === 'SettingsUpdateSafety' ? 'active' : ''}`}
                  aria-label="Update Safety"
                  title="Update Safety"
                  onClick={() => navigateToPage('SettingsUpdateSafety')}
                >
                  <ShieldCheck size={15} />
                  <span>Update Safety</span>
                </button>
              </div>
            )}
          </div>
        </nav>

      </aside>

      <section className="workspace">
        <header className="topbar">
          <div className="topbar-title">
            <p className="eyebrow">{pageEyebrow}</p>
            <h1>{pageTitle}</h1>
          </div>
        </header>

        {activePage === 'Dashboard' && (
          <section className="metric-grid" aria-label="Fleet metrics">
            <MetricTile icon={Radio} tone="success" label="Online" value={onlineCount} />
            <MetricTile icon={AlertTriangle} tone="warning" label="Needs review" value={reviewCount} />
            <MetricTile icon={Network} tone="danger" label="Offline" value={offlineCount} />
            <MetricTile icon={Gauge} tone="neutral" label="Total" value={receivers.length || pendingCount} />
          </section>
        )}

        {(activePage === 'Dashboard' || activePage === 'Receivers') && (
          <div
            className={`content-grid ${activePage === 'Receivers' ? 'receivers-full-grid' : 'dashboard-content-grid'}`}
          >
            <section
              className={`surface receiver-surface ${activePage === 'Dashboard' ? 'dashboard-alert-surface' : ''}`}
            >
              <div className="surface-header receiver-header">
                <div>
                  <h2>{receiverSectionTitle}</h2>
                  <p>{receiverSectionSubtitle}</p>
                </div>
                {activePage === 'Receivers' && (
                  <label className="search-box section-search">
                    <Search size={17} />
                    <input
                      type="search"
                      value={query}
                      onChange={(event) => setQuery(event.target.value)}
                      placeholder="Search IP, firmware, fix type"
                    />
                  </label>
                )}
                <div className="receiver-tools">
                  <button className="secondary-button" onClick={() => void scanReceivers()}>
                    <RefreshCw size={16} />
                    Refresh
                  </button>
                </div>
              </div>

              {activePage === 'Dashboard' ? (
                <DashboardOfflineList
                  receivers={visibleReceivers}
                  onOpen={openReceiver}
                  onShowLoginDetails={openLoginDetails}
                />
              ) : (
                <>
                  <div className="receiver-filter-bar">
                    <label>
                      <span>FW</span>
                      <select value={receiverFwFilter} onChange={(event) => setReceiverFwFilter(event.target.value)}>
                        <option value="all">All firmware</option>
                        {receiverFirmwareFilterOptions.map((option) => (
                          <option value={option} key={option}>
                            {option}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label>
                      <span>FIX</span>
                      <select value={receiverFixFilter} onChange={(event) => setReceiverFixFilter(event.target.value)}>
                        <option value="all">All fix types</option>
                        {receiverFixFilterOptions.map((option) => (
                          <option value={option} key={option}>
                            {option}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label>
                      <span>STATUS</span>
                      <select
                        value={receiverStatusFilter}
                        onChange={(event) => setReceiverStatusFilter(event.target.value)}
                      >
                        <option value="all">All statuses</option>
                        <option value="online">Online</option>
                        <option value="offline">Offline</option>
                      </select>
                    </label>
                  </div>

                  <div className="receiver-list">
                    {visibleReceivers.map((receiver) => (
                      <ReceiverRow
                        receiver={receiver}
                        onOpen={openReceiver}
                        onShowLoginDetails={openLoginDetails}
                        key={receiver.ip}
                      />
                    ))}
                  </div>
                </>
              )}
            </section>

            {activePage === 'Dashboard' && (
              <aside className="right-stack">
                <section className="surface log-surface">
                  <div className="surface-header compact">
                    <h2>Live Log</h2>
                    <span className="log-badge">live</span>
                  </div>
                  <div className="log-lines" role="log" aria-live="polite">
                    {logLines.map((line, index) => {
                      const match = line.match(/^\[([^\]]+)]\s*(.*)$/)
                      return (
                        <div className="log-entry" key={`${line}-${index}`}>
                          <time>{match?.[1] || 'SYSTEM'}</time>
                          <span>{match?.[2] || line}</span>
                        </div>
                      )
                    })}
                  </div>
                </section>
              </aside>
            )}
          </div>
        )}

        <div className={`firmware-route ${isFirmwareActive ? 'active' : 'hidden'}`}>
          <FirmwareModulePage
            page={isFirmwareActive ? (activePage as FirmwarePage) : lastFirmwarePage}
            receivers={receivers}
            onActivity={addLog}
          />
        </div>

        {activePage === 'History' && <HistoryPage />}

        {activePage === 'SettingsUpdateSafety' && <UpdateSafetyPage onActivity={addLog} />}

        {activePage !== 'Dashboard' &&
          activePage !== 'Receivers' &&
          activePage !== 'History' &&
          !isFirmwareActive &&
          !isSettingsActive && (
          <section className="surface placeholder-surface">
            <h2>{activePage}</h2>
            <p>This module will be connected in the next milestones.</p>
          </section>
          )}

        {loginDetailsTarget && (
          <LoginDetailsModal
            receiver={loginDetailsTarget}
            password={loginDetailsPassword}
            showPassword={showLoginDetailsPassword}
            details={loginDetails}
            error={loginDetailsError}
            isLoading={isLoginDetailsLoading}
            showReceiverPassword={showReceiverPassword}
            onPasswordChange={setLoginDetailsPassword}
            onTogglePassword={() => setShowLoginDetailsPassword((current) => !current)}
            onToggleReceiverPassword={() => setShowReceiverPassword((current) => !current)}
            onSubmit={loadLoginDetails}
            onClose={closeLoginDetails}
          />
        )}
      </section>
    </main>
  )
}

function FirmwareModulePage({
  page,
  receivers,
  onActivity,
}: {
  page: FirmwarePage
  receivers: Receiver[]
  onActivity: (message: string) => void
}) {
  const metadata: Record<
    FirmwarePage,
    { title: string; subtitle: string; icon: typeof UploadCloud; mode?: 'Alpha' | 'Beta' }
  > = {
    FirmwareAlpha: {
      title: 'Alpha Receivers',
      subtitle: 'Group firmware update workspace for Alpha receivers.',
      icon: Radio,
      mode: 'Alpha',
    },
    FirmwareBeta: {
      title: 'Beta Receivers',
      subtitle: 'Group firmware update workspace for Beta receivers.',
      icon: Radio,
      mode: 'Beta',
    },
    FirmwareCreateGroup: {
      title: 'Create New Group',
      subtitle: 'Prepare a receiver group for future firmware operations.',
      icon: Plus,
    },
    FirmwareSingle: {
      title: 'Single Receiver Update',
      subtitle: 'Firmware update workspace for one selected receiver.',
      icon: UploadCloud,
    },
    FirmwareMultiple: {
      title: 'Multiple Receivers Update',
      subtitle: 'Install one firmware release across a controlled receiver selection.',
      icon: UploadCloud,
    },
  }
  const current = metadata[page]
  const Icon = current.icon
  const isGroupUpdate = page === 'FirmwareAlpha' || page === 'FirmwareBeta'
  const [firmwareReceivers, setFirmwareReceivers] = useState<FirmwareReceiver[]>([])
  const [loadedFirmwareMode, setLoadedFirmwareMode] = useState('')
  const [selectedIps, setSelectedIps] = useState<Set<string>>(new Set())
  const [groupReleaseMode, setGroupReleaseMode] = useState<'catalog' | 'local'>('catalog')
  const [groupLocalPackages, setGroupLocalPackages] = useState<Record<string, LocalFirmwarePackage>>({})
  const [groupPackageError, setGroupPackageError] = useState('')
  const [isGroupPackageLoading, setIsGroupPackageLoading] = useState(false)
  const [isGroupPackageDragging, setIsGroupPackageDragging] = useState(false)
  const [releaseCatalog, setReleaseCatalog] = useState<FirmwareReleaseCatalog | null>(null)
  const [selectedReleaseId, setSelectedReleaseId] = useState('')
  const [selectedReleaseMode, setSelectedReleaseMode] = useState('')
  const [buildCatalog, setBuildCatalog] = useState<FirmwareBuildCatalog | null>(null)
  const [selectedBuildId, setSelectedBuildId] = useState('')
  const [releaseInspection, setReleaseInspection] = useState<FirmwareReleaseInspection | null>(null)
  const [releaseCatalogError, setReleaseCatalogError] = useState('')
  const [releaseCatalogErrorMode, setReleaseCatalogErrorMode] = useState('')
  const [isReleaseCatalogLoading, setIsReleaseCatalogLoading] = useState(false)
  const [isBuildCatalogLoading, setIsBuildCatalogLoading] = useState(false)
  const [isReleaseInspectionLoading, setIsReleaseInspectionLoading] = useState(false)
  const [releaseCatalogRevision, setReleaseCatalogRevision] = useState(0)
  const [standaloneSelectedReleaseId, setStandaloneSelectedReleaseId] = useState('')
  const [standaloneBuildOptions, setStandaloneBuildOptions] = useState<StandaloneFirmwareBuild[]>([])
  const [standaloneSelectedBuildId, setStandaloneSelectedBuildId] = useState('')
  const [standaloneSelectedBuildMode, setStandaloneSelectedBuildMode] = useState<'Alpha' | 'Beta' | ''>('')
  const [standaloneReleaseInspection, setStandaloneReleaseInspection] =
    useState<FirmwareReleaseInspection | null>(null)
  const [standaloneCatalogError, setStandaloneCatalogError] = useState('')
  const [isStandaloneBuildLoading, setIsStandaloneBuildLoading] = useState(false)
  const [isStandaloneInspectionLoading, setIsStandaloneInspectionLoading] = useState(false)
  const [newReceiverIp, setNewReceiverIp] = useState('')
  const [newReceiverModel, setNewReceiverModel] = useState('')
  const [isAddReceiverModalOpen, setIsAddReceiverModalOpen] = useState(false)
  const [receiverPendingModelEdit, setReceiverPendingModelEdit] = useState<FirmwareReceiver | null>(null)
  const [receiverPendingRemoval, setReceiverPendingRemoval] = useState<FirmwareReceiver | null>(null)
  const [removePassword, setRemovePassword] = useState('')
  const [showRemovePassword, setShowRemovePassword] = useState(false)
  const [parallel, setParallel] = useState(false)
  const [isLoadingReceivers, setIsLoadingReceivers] = useState(false)
  const [isJobStarting, setIsJobStarting] = useState(false)
  const [isStoppingGroupJob, setIsStoppingGroupJob] = useState(false)
  const [isRestoringGroupPasswords, setIsRestoringGroupPasswords] = useState(false)
  const [groupPasswordRecoveryResult, setGroupPasswordRecoveryResult] =
    useState<FirmwarePasswordRecoveryBatchResult | null>(null)
  const [resolvingDowngradeJobIds, setResolvingDowngradeJobIds] = useState<Set<string>>(new Set())
  const [isSavingReceiver, setIsSavingReceiver] = useState(false)
  const [firmwareError, setFirmwareError] = useState('')
  const [jobStatus, setJobStatus] = useState<FirmwareJobStatus | null>(null)
  const [isGroupStartModalOpen, setIsGroupStartModalOpen] = useState(false)
  const [isGroupJobMinimized, setIsGroupJobMinimized] = useState(false)
  const [expandedGroupIps, setExpandedGroupIps] = useState<Set<string>>(new Set())
  const [singleReceiverIp, setSingleReceiverIp] = useState('')
  const [singleReceiverModel, setSingleReceiverModel] = useState('')
  const [singleSourceMode, setSingleSourceMode] = useState<'release' | 'local'>('release')
  const [singleFirmwareFile, setSingleFirmwareFile] = useState<SelectedFirmwareFile | null>(null)
  const [singleFirmwareFilePath, setSingleFirmwareFilePath] = useState('')
  const [isFirmwareFileDragging, setIsFirmwareFileDragging] = useState(false)
  const [singleFileError, setSingleFileError] = useState('')
  const [isSingleFileLoading, setIsSingleFileLoading] = useState(false)
  const [isSingleUpdateStarting, setIsSingleUpdateStarting] = useState(false)
  const [singleUpdateJobs, setSingleUpdateJobs] = useState<FirmwareSingleUpdateJobStatus[]>([])
  const [activeSingleUpdateJobId, setActiveSingleUpdateJobId] = useState<string | null>(null)
  const [minimizedSingleUpdateJobIds, setMinimizedSingleUpdateJobIds] = useState<Set<string>>(new Set())
  const [isPasswordRestoreRunning, setIsPasswordRestoreRunning] = useState(false)
  const [passwordRestoreResult, setPasswordRestoreResult] = useState<PasswordRestoreResult | null>(null)
  const [multipleSelectedIps, setMultipleSelectedIps] = useState<Set<string>>(new Set())
  const [multipleReceiverModels, setMultipleReceiverModels] = useState<Record<string, string>>({})
  const [multipleQuery, setMultipleQuery] = useState('')
  const [multipleSourceMode, setMultipleSourceMode] = useState<'release' | 'local'>('release')
  const [multipleFirmwareFile, setMultipleFirmwareFile] = useState<SelectedFirmwareFile | null>(null)
  const [multipleFirmwareFilePath, setMultipleFirmwareFilePath] = useState('')
  const [isMultipleFileDragging, setIsMultipleFileDragging] = useState(false)
  const [isMultipleFileLoading, setIsMultipleFileLoading] = useState(false)
  const [multipleFileError, setMultipleFileError] = useState('')
  const [multipleBatch, setMultipleBatch] = useState<MultipleUpdateBatch | null>(null)
  const [isMultipleBatchMinimized, setIsMultipleBatchMinimized] = useState(false)
  const [expandedMultipleIps, setExpandedMultipleIps] = useState<Set<string>>(new Set())
  const [isMultipleUpdateStarting, setIsMultipleUpdateStarting] = useState(false)
  const [newGroupName, setNewGroupName] = useState('')
  const [newGroupQuery, setNewGroupQuery] = useState('')
  const [newGroupSelectedIps, setNewGroupSelectedIps] = useState<Set<string>>(new Set())
  const firmwareFileInputRef = useRef<HTMLInputElement>(null)
  const multipleFirmwareFileInputRef = useRef<HTMLInputElement>(null)
  const reportedSingleActivityRef = useRef<Map<string, string>>(new Map())
  const reportedMultipleActivityRef = useRef('')
  const reportedGroupActivityRef = useRef('')
  const releaseCatalogRequestRef = useRef(0)

  const selectedList = useMemo(() => Array.from(selectedIps).sort(), [selectedIps])
  const receiverStatusByIp = useMemo(() => new Map(receivers.map((receiver) => [receiver.ip, receiver])), [receivers])
  const visibleFirmwareReceivers = useMemo(
    () => (current.mode === loadedFirmwareMode ? firmwareReceivers : []),
    [current.mode, firmwareReceivers, loadedFirmwareMode],
  )
  const configuredIpSet = useMemo(
    () =>
      new Set(
        visibleFirmwareReceivers
          .filter((receiver) => receiver.model.trim().toLowerCase() !== 'unknown')
          .map((receiver) => receiver.ip),
      ),
    [visibleFirmwareReceivers],
  )
  const receiverPickerOptions = useMemo(() => {
    const needle = newReceiverIp.trim().toLowerCase()
    const expectedGroup = current.mode === 'Alpha' ? 2 : current.mode === 'Beta' ? 1 : null

    return receivers
      .filter((receiver) => {
        if (expectedGroup !== null && receiver.group !== expectedGroup) {
          return false
        }
        const haystack = [receiver.ip, receiver.group, receiver.receiverName, receiver.firmwareVersion]
          .filter(Boolean)
          .join(' ')
          .toLowerCase()

        return !needle || haystack.includes(needle)
      })
  }, [current.mode, newReceiverIp, receivers])
  const configuredGroupModels = useMemo(
    () => Array.from(new Set(visibleFirmwareReceivers.map((receiver) => receiver.model.trim().toLowerCase()))).sort(),
    [visibleFirmwareReceivers],
  )
  const visibleReleaseOptions = useMemo(() => {
    const blockedChannel = current.mode === 'Alpha' ? 'beta' : current.mode === 'Beta' ? 'alpha' : ''
    if (!releaseCatalog || !blockedChannel) {
      return releaseCatalog?.releases || []
    }
    return releaseCatalog.releases.filter((release) => !release.name.toLowerCase().includes(blockedChannel))
  }, [current.mode, releaseCatalog])
  const standaloneReleaseOptions = releaseCatalog?.releases || []
  const standaloneReleaseMatchesByModel = useMemo(
    () => new Map((standaloneReleaseInspection?.modelMatches || []).map((match) => [match.model, match])),
    [standaloneReleaseInspection],
  )
  const activeSelectedReleaseId = selectedReleaseMode === current.mode ? selectedReleaseId : ''
  const activeSelectedBuildId = selectedReleaseMode === current.mode ? selectedBuildId : ''
  const isCurrentBuildCatalogLoading = selectedReleaseMode === current.mode && isBuildCatalogLoading
  const isCurrentReleaseInspectionLoading = selectedReleaseMode === current.mode && isReleaseInspectionLoading
  const visibleReleaseCatalogError =
    !releaseCatalogError || releaseCatalogErrorMode === current.mode ? releaseCatalogError : ''
  const selectedGroupModels = useMemo(
    () =>
      Array.from(
        new Set(
          visibleFirmwareReceivers
            .filter((receiver) => selectedIps.has(receiver.ip))
            .map((receiver) => receiver.model.trim().toLowerCase()),
        ),
      ).sort(),
    [selectedIps, visibleFirmwareReceivers],
  )
  const currentReleaseInspection = useMemo(
    () =>
      selectedReleaseMode === current.mode &&
      releaseInspection?.releaseId === activeSelectedReleaseId &&
      releaseInspection.buildId === activeSelectedBuildId
        ? releaseInspection
        : null,
    [activeSelectedBuildId, activeSelectedReleaseId, current.mode, releaseInspection, selectedReleaseMode],
  )
  const releaseMatchesByModel = useMemo(
    () => new Map((currentReleaseInspection?.modelMatches || []).map((match) => [match.model, match])),
    [currentReleaseInspection],
  )
  const releasePackageOverrides = useMemo(() => {
    const packages: Record<string, string> = {}
    selectedGroupModels.forEach((model) => {
      const match = releaseMatchesByModel.get(model)
      if (match?.status === 'Matched' && match.selectedFile) {
        packages[model] = match.selectedFile
      }
    })
    return packages
  }, [releaseMatchesByModel, selectedGroupModels])
  const readyConfiguredModelCount = configuredGroupModels.filter((model) => {
    const match = releaseMatchesByModel.get(model)
    return match?.status === 'Matched' && Boolean(match.selectedFile)
  }).length
  const configuredPackagesReady =
    configuredGroupModels.length > 0 && readyConfiguredModelCount === configuredGroupModels.length
  const catalogPackagesReady =
    selectedGroupModels.length > 0 &&
    selectedGroupModels.every((model) => {
      const match = releaseMatchesByModel.get(model)
      return match?.status === 'Matched' && Boolean(match.selectedFile)
    })
  const groupHasMissingModels = selectedGroupModels.some((model) => !RECEIVER_MODEL_OPTIONS.includes(model))
  const groupLocalPackagesReady =
    selectedGroupModels.length > 0 &&
    !groupHasMissingModels &&
    selectedGroupModels.every((model) => Boolean(groupLocalPackages[model]?.path))
  const hasActiveGroupJob =
    Boolean(jobStatus) &&
    jobStatus?.status !== 'Completed' &&
    jobStatus?.status !== 'Failed' &&
    jobStatus?.status !== 'Cancelled'
  const canRunPlan =
    isGroupUpdate &&
    selectedIps.size > 0 &&
    !hasActiveGroupJob &&
    (groupReleaseMode === 'catalog'
      ? Boolean(activeSelectedReleaseId) &&
        Boolean(activeSelectedBuildId) &&
        catalogPackagesReady &&
        !isCurrentBuildCatalogLoading &&
        !isCurrentReleaseInspectionLoading
      : groupLocalPackagesReady && !isGroupPackageLoading)
  const singleReceiverOptions = useMemo(() => {
    const needle = singleReceiverIp.trim().toLowerCase()
    const exactReceiver = receivers.find((receiver) => receiver.ip.toLowerCase() === needle)

    if (exactReceiver) {
      return [exactReceiver]
    }

    return receivers
      .filter((receiver) => {
        const haystack = [receiver.ip, receiver.group, receiver.receiverName, receiver.firmwareVersion]
          .filter(Boolean)
          .join(' ')
          .toLowerCase()

        return !needle || haystack.includes(needle)
      })
  }, [receivers, singleReceiverIp])
  const selectedSingleReceiver = useMemo(
    () => receivers.find((receiver) => receiver.ip === singleReceiverIp.trim()) ?? null,
    [receivers, singleReceiverIp],
  )
  const effectiveSingleReceiverModel = (
    singleReceiverModel || (selectedSingleReceiver ? multipleReceiverModels[selectedSingleReceiver.ip] : '') || ''
  )
    .trim()
    .toLowerCase()
  const activeSingleUpdateJobs = useMemo(
    () => singleUpdateJobs.filter((job) => job.status !== 'Completed' && job.status !== 'Failed'),
    [singleUpdateJobs],
  )
  const activeMultipleUpdateItems =
    multipleBatch?.items.filter((item) => item.status === 'Starting' || item.status === 'Running') || []
  const canStartAnotherSingleUpdate = activeSingleUpdateJobs.length + activeMultipleUpdateItems.length < 3
  const selectedReceiverHasRunningUpdate =
    activeSingleUpdateJobs.some((job) => job.ip === singleReceiverIp.trim()) ||
    activeMultipleUpdateItems.some((item) => item.ip === singleReceiverIp.trim())
  const singleCatalogPackage =
    standaloneReleaseMatchesByModel.get(effectiveSingleReceiverModel)?.selectedFile || ''
  const canStartSingleReleaseUpdate =
    singleSourceMode === 'release' &&
    Boolean(selectedSingleReceiver) &&
    RECEIVER_MODEL_OPTIONS.includes(effectiveSingleReceiverModel) &&
    Boolean(standaloneSelectedReleaseId) &&
    Boolean(standaloneSelectedBuildId) &&
    Boolean(standaloneSelectedBuildMode) &&
    Boolean(singleCatalogPackage) &&
    !isStandaloneBuildLoading &&
    !isStandaloneInspectionLoading &&
    canStartAnotherSingleUpdate &&
    !selectedReceiverHasRunningUpdate &&
    !isSingleUpdateStarting
  const canStartSingleLocalUpdate =
    singleSourceMode === 'local' &&
    Boolean(selectedSingleReceiver) &&
    Boolean(singleFirmwareFile) &&
    Boolean(singleFirmwareFilePath) &&
    canStartAnotherSingleUpdate &&
    !selectedReceiverHasRunningUpdate &&
    !isSingleUpdateStarting &&
    !isSingleFileLoading
  const multipleReceiverOptions = useMemo(() => {
    const needle = multipleQuery.trim().toLowerCase()
    return receivers.filter((receiver) => {
      const model = multipleReceiverModels[receiver.ip] || ''
      const haystack = [receiver.ip, receiver.receiverName, receiver.group, receiver.firmwareVersion, model]
        .filter(Boolean)
        .join(' ')
        .toLowerCase()
      return !needle || haystack.includes(needle)
    })
  }, [multipleQuery, multipleReceiverModels, receivers])
  const multipleSelectedReceivers = useMemo(
    () => receivers.filter((receiver) => multipleSelectedIps.has(receiver.ip)),
    [multipleSelectedIps, receivers],
  )
  const newGroupReceiverOptions = useMemo(() => {
    const needle = newGroupQuery.trim().toLowerCase()
    return receivers.filter((receiver) => {
      const haystack = [receiver.ip, receiver.receiverName, receiver.group, receiver.firmwareVersion, receiver.fixType]
        .filter(Boolean)
        .join(' ')
        .toLowerCase()
      return !needle || haystack.includes(needle)
    })
  }, [newGroupQuery, receivers])
  const multipleSelectedModels = useMemo(
    () =>
      new Set(
        multipleSelectedReceivers
          .map((receiver) => (multipleReceiverModels[receiver.ip] || '').trim().toLowerCase())
          .filter(Boolean),
      ),
    [multipleReceiverModels, multipleSelectedReceivers],
  )
  const multipleSharedLocalModel =
    multipleSelectedModels.size === 1 ? Array.from(multipleSelectedModels)[0] : ''
  const multipleHasMissingModels = multipleSelectedReceivers.some(
    (receiver) => !RECEIVER_MODEL_OPTIONS.includes((multipleReceiverModels[receiver.ip] || '').trim().toLowerCase()),
  )
  const multipleCatalogPackages = useMemo(() => {
    const packages: Record<string, string> = {}
    multipleSelectedModels.forEach((model) => {
      const match = standaloneReleaseMatchesByModel.get(model)
      if (match?.status === 'Matched' && match.selectedFile) {
        packages[model] = match.selectedFile
      }
    })
    return packages
  }, [multipleSelectedModels, standaloneReleaseMatchesByModel])
  const multipleCatalogPackagesReady =
    multipleSelectedModels.size > 0 &&
    Array.from(multipleSelectedModels).every((model) => Boolean(multipleCatalogPackages[model]))
  const standaloneInspectionModels = useMemo(() => {
    if (page === 'FirmwareSingle') {
      const model = effectiveSingleReceiverModel
      return RECEIVER_MODEL_OPTIONS.includes(model) ? [model] : []
    }
    if (page === 'FirmwareMultiple') {
      return Array.from(multipleSelectedModels).filter((model) => RECEIVER_MODEL_OPTIONS.includes(model)).sort()
    }
    return []
  }, [effectiveSingleReceiverModel, multipleSelectedModels, page])
  const hasRunningMultipleBatch = multipleBatch?.status === 'Running'
  const canStartMultipleReleaseUpdate =
    multipleSourceMode === 'release' &&
    multipleSelectedReceivers.length >= 2 &&
    !multipleHasMissingModels &&
    Boolean(standaloneSelectedReleaseId) &&
    Boolean(standaloneSelectedBuildId) &&
    Boolean(standaloneSelectedBuildMode) &&
    multipleCatalogPackagesReady &&
    !isStandaloneBuildLoading &&
    !isStandaloneInspectionLoading &&
    !hasRunningMultipleBatch &&
    activeSingleUpdateJobs.length + activeMultipleUpdateItems.length < 3 &&
    !isMultipleUpdateStarting
  const canStartMultipleLocalUpdate =
    multipleSourceMode === 'local' &&
    multipleSelectedReceivers.length >= 2 &&
    !multipleHasMissingModels &&
    multipleSelectedModels.size === 1 &&
    Boolean(multipleFirmwareFile) &&
    Boolean(multipleFirmwareFilePath) &&
    !hasRunningMultipleBatch &&
    activeSingleUpdateJobs.length + activeMultipleUpdateItems.length < 3 &&
    !isMultipleUpdateStarting &&
    !isMultipleFileLoading

  const loadReleaseCatalog = useCallback(async () => {
    if (!isTauri()) {
      return
    }

    const requestId = releaseCatalogRequestRef.current + 1
    releaseCatalogRequestRef.current = requestId
    try {
      setIsReleaseCatalogLoading(true)
      setReleaseCatalogError('')
      setReleaseCatalogErrorMode('')
      const catalog = await invoke<FirmwareReleaseCatalog>('list_firmware_releases')
      if (requestId !== releaseCatalogRequestRef.current) {
        return
      }
      setReleaseCatalog(catalog)
      setSelectedReleaseId('')
      setSelectedReleaseMode('')
      setBuildCatalog(null)
      setSelectedBuildId('')
      setReleaseInspection(null)
      setReleaseCatalogRevision((current) => current + 1)
    } catch (error) {
      if (requestId !== releaseCatalogRequestRef.current) {
        return
      }
      setReleaseCatalog(null)
      setSelectedReleaseMode('')
      setBuildCatalog(null)
      setSelectedBuildId('')
      setReleaseInspection(null)
      setReleaseCatalogError(formatError(error))
      setReleaseCatalogErrorMode(current.mode || '')
      setGroupReleaseMode('local')
    } finally {
      if (requestId === releaseCatalogRequestRef.current) {
        setIsReleaseCatalogLoading(false)
      }
    }
  }, [current.mode])

  useEffect(() => {
    if (
      !isGroupUpdate ||
      groupReleaseMode !== 'catalog' ||
      !activeSelectedReleaseId ||
      selectedReleaseMode !== current.mode ||
      !isTauri()
    ) {
      return undefined
    }

    let cancelled = false
    void (async () => {
      try {
        setIsBuildCatalogLoading(true)
        setReleaseCatalogError('')
        setReleaseCatalogErrorMode('')
        const catalog = await invoke<FirmwareBuildCatalog>('list_firmware_builds', {
          releaseId: activeSelectedReleaseId,
          mode: current.mode,
        })
        if (!cancelled) {
          setBuildCatalog(catalog)
          setSelectedBuildId('')
          setReleaseInspection(null)
        }
      } catch (error) {
        if (!cancelled) {
          setBuildCatalog(null)
          setSelectedBuildId('')
          setReleaseInspection(null)
          setReleaseCatalogError(formatError(error))
          setReleaseCatalogErrorMode(current.mode || '')
        }
      } finally {
        if (!cancelled) {
          setIsBuildCatalogLoading(false)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [activeSelectedReleaseId, current.mode, groupReleaseMode, isGroupUpdate, releaseCatalogRevision, selectedReleaseMode])

  useEffect(() => {
    if (
      !isGroupUpdate ||
      groupReleaseMode !== 'catalog' ||
      !activeSelectedReleaseId ||
      !activeSelectedBuildId ||
      selectedReleaseMode !== current.mode ||
      !configuredGroupModels.length ||
      !current.mode ||
      currentReleaseInspection ||
      !isTauri()
    ) {
      return
    }

    let cancelled = false
    void (async () => {
      try {
        setIsReleaseInspectionLoading(true)
        setReleaseCatalogError('')
        setReleaseCatalogErrorMode('')
        const inspection = await invoke<FirmwareReleaseInspection>('inspect_firmware_release', {
          releaseId: activeSelectedReleaseId,
          buildId: activeSelectedBuildId,
          mode: current.mode,
          models: configuredGroupModels,
        })
        if (!cancelled) {
          setReleaseInspection(inspection)
        }
      } catch (error) {
        if (!cancelled) {
          setReleaseInspection(null)
          setReleaseCatalogError(formatError(error))
          setReleaseCatalogErrorMode(current.mode || '')
        }
      } finally {
        if (!cancelled) {
          setIsReleaseInspectionLoading(false)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [
    configuredGroupModels,
    current.mode,
    currentReleaseInspection,
    groupReleaseMode,
    isGroupUpdate,
    activeSelectedBuildId,
    activeSelectedReleaseId,
    selectedReleaseMode,
  ])

  useEffect(() => {
    const usesReleaseCatalog =
      (page === 'FirmwareSingle' && singleSourceMode === 'release') ||
      (page === 'FirmwareMultiple' && multipleSourceMode === 'release')
    if (!usesReleaseCatalog || releaseCatalog || isReleaseCatalogLoading || !isTauri()) {
      return
    }
    void (async () => {
      await Promise.resolve()
      await loadReleaseCatalog()
    })()
  }, [
    isReleaseCatalogLoading,
    loadReleaseCatalog,
    multipleSourceMode,
    page,
    releaseCatalog,
    singleSourceMode,
  ])

  useEffect(() => {
    if (
      (page !== 'FirmwareSingle' && page !== 'FirmwareMultiple') ||
      !standaloneSelectedReleaseId ||
      !isTauri()
    ) {
      return undefined
    }

    let cancelled = false
    void (async () => {
      try {
        setIsStandaloneBuildLoading(true)
        setStandaloneCatalogError('')
        const results = await Promise.allSettled(
          (['Alpha', 'Beta'] as const).map(async (mode) => {
            const catalog = await invoke<FirmwareBuildCatalog>('list_firmware_builds', {
              releaseId: standaloneSelectedReleaseId,
              mode,
            })
            return catalog.builds.map((build) => ({ ...build, mode }))
          }),
        )
        const builds = results
          .flatMap((result) => (result.status === 'fulfilled' ? result.value : []))
          .filter(
            (build, index, all) =>
              all.findIndex((candidate) => candidate.mode === build.mode && candidate.id === build.id) === index,
          )
        if (!builds.length) {
          const errors = results
            .filter((result): result is PromiseRejectedResult => result.status === 'rejected')
            .map((result) => formatError(result.reason))
          throw new Error(errors[0] || 'The selected release does not contain Alpha or Beta builds.')
        }
        if (!cancelled) {
          setStandaloneBuildOptions(builds)
          setStandaloneSelectedBuildId('')
          setStandaloneSelectedBuildMode('')
          setStandaloneReleaseInspection(null)
        }
      } catch (error) {
        if (!cancelled) {
          setStandaloneBuildOptions([])
          setStandaloneSelectedBuildId('')
          setStandaloneSelectedBuildMode('')
          setStandaloneReleaseInspection(null)
          setStandaloneCatalogError(formatError(error))
        }
      } finally {
        if (!cancelled) {
          setIsStandaloneBuildLoading(false)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [page, standaloneSelectedReleaseId])

  useEffect(() => {
    if (
      (page !== 'FirmwareSingle' && page !== 'FirmwareMultiple') ||
      !standaloneSelectedReleaseId ||
      !standaloneSelectedBuildId ||
      !standaloneSelectedBuildMode ||
      !standaloneInspectionModels.length ||
      !isTauri()
    ) {
      return undefined
    }

    let cancelled = false
    void (async () => {
      try {
        setIsStandaloneInspectionLoading(true)
        setStandaloneCatalogError('')
        const inspection = await invoke<FirmwareReleaseInspection>('inspect_firmware_release', {
          releaseId: standaloneSelectedReleaseId,
          buildId: standaloneSelectedBuildId,
          mode: standaloneSelectedBuildMode,
          models: standaloneInspectionModels,
        })
        if (!cancelled) {
          setStandaloneReleaseInspection(inspection)
        }
      } catch (error) {
        if (!cancelled) {
          setStandaloneReleaseInspection(null)
          setStandaloneCatalogError(formatError(error))
        }
      } finally {
        if (!cancelled) {
          setIsStandaloneInspectionLoading(false)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [
    page,
    standaloneInspectionModels,
    standaloneSelectedBuildId,
    standaloneSelectedBuildMode,
    standaloneSelectedReleaseId,
  ])

  const loadFirmwareReceivers = useCallback(async () => {
    if (!current.mode) {
      return
    }

    try {
      setIsLoadingReceivers(true)
      setFirmwareError('')
      const config = await invoke<FirmwareConfigResponse>('get_firmware_update_config', { mode: current.mode })
      setFirmwareReceivers(config.receivers)
      setLoadedFirmwareMode(current.mode)
      setSelectedIps(new Set(config.receivers.map((receiver) => receiver.ip)))
    } catch (error) {
      setFirmwareReceivers([])
      setLoadedFirmwareMode('')
      setSelectedIps(new Set())
      setFirmwareError(formatError(error))
    } finally {
      setIsLoadingReceivers(false)
    }
  }, [current.mode])

  useEffect(() => {
    let cancelled = false
    if (!current.mode) {
      return undefined
    }
    const mode = current.mode

    void (async () => {
      try {
        const config = await invoke<FirmwareConfigResponse>('get_firmware_update_config', { mode })
        if (cancelled) {
          return
        }
        setFirmwareError('')
        setReleaseCatalogError('')
        setReleaseCatalogErrorMode('')
        setFirmwareReceivers(config.receivers)
        setLoadedFirmwareMode(mode)
        setSelectedIps(new Set(config.receivers.map((receiver) => receiver.ip)))
        setSelectedReleaseId('')
        setSelectedReleaseMode('')
        setBuildCatalog(null)
        setSelectedBuildId('')
        setReleaseInspection(null)
        await loadReleaseCatalog()
      } catch (error) {
        if (cancelled) {
          return
        }
        setFirmwareReceivers([])
        setLoadedFirmwareMode('')
        setSelectedIps(new Set())
        setFirmwareError(formatError(error))
      }
    })()

    return () => {
      cancelled = true
    }
  }, [current.mode, loadReleaseCatalog])

  useEffect(() => {
    if ((page !== 'FirmwareSingle' && page !== 'FirmwareMultiple') || !isTauri()) {
      return undefined
    }

    let cancelled = false
    void (async () => {
      try {
        const [alpha, beta] = await Promise.all([
          invoke<FirmwareConfigResponse>('get_firmware_update_config', { mode: 'Alpha' }),
          invoke<FirmwareConfigResponse>('get_firmware_update_config', { mode: 'Beta' }),
        ])
        if (cancelled) {
          return
        }

        const models: Record<string, string> = {}
        ;[...alpha.receivers, ...beta.receivers].forEach((receiver) => {
          models[receiver.ip] = receiver.model
        })
        setMultipleReceiverModels((current) => ({ ...models, ...current }))
      } catch (error) {
        if (!cancelled) {
          setFirmwareError(formatError(error))
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [page])

  const buildPlan = useCallback((): FirmwarePlanRequest | null => {
    if (!current.mode) {
      return null
    }

    return {
      mode: current.mode,
      firmwareVersion: groupReleaseMode === 'catalog' ? activeSelectedReleaseId : 'Local packages',
      firmwareSuffix: '',
      receiverIps: selectedList,
      parallel,
      packageOverrides: groupReleaseMode === 'catalog' ? releasePackageOverrides : undefined,
      localPackagePaths:
        groupReleaseMode === 'local'
          ? Object.fromEntries(selectedGroupModels.map((model) => [model, groupLocalPackages[model].path]))
          : undefined,
    }
  }, [
    current.mode,
    groupLocalPackages,
    groupReleaseMode,
    parallel,
    releasePackageOverrides,
    selectedGroupModels,
    selectedList,
    activeSelectedReleaseId,
  ])

  const addReceiver = useCallback(async () => {
    if (!current.mode) {
      return
    }

    try {
      setIsSavingReceiver(true)
      setFirmwareError('')
      const config = await invoke<FirmwareConfigResponse>('add_firmware_config_receiver', {
        mode: current.mode,
        ip: newReceiverIp,
        model: newReceiverModel,
      })
      setFirmwareReceivers(config.receivers)
      setSelectedIps(new Set(config.receivers.map((receiver) => receiver.ip)))
      setNewReceiverIp('')
      setNewReceiverModel('')
      setReceiverPendingModelEdit(null)
      setIsAddReceiverModalOpen(false)
      setJobStatus(null)
    } catch (error) {
      setFirmwareError(formatError(error))
    } finally {
      setIsSavingReceiver(false)
    }
  }, [current.mode, newReceiverIp, newReceiverModel])

  const removeReceiver = useCallback(
    async () => {
      if (!current.mode) {
        return
      }

      if (!receiverPendingRemoval) {
        return
      }

      try {
        setIsSavingReceiver(true)
        setFirmwareError('')
        const config = await invoke<FirmwareConfigResponse>('remove_firmware_config_receiver', {
          mode: current.mode,
          ip: receiverPendingRemoval.ip,
          appPassword: removePassword,
        })
        setFirmwareReceivers(config.receivers)
        setSelectedIps(new Set(config.receivers.map((receiver) => receiver.ip)))
        setReceiverPendingRemoval(null)
        setRemovePassword('')
        setShowRemovePassword(false)
        setJobStatus(null)
      } catch (error) {
        setFirmwareError(formatError(error))
      } finally {
        setIsSavingReceiver(false)
      }
    },
    [current.mode, receiverPendingRemoval, removePassword],
  )

  const prepareJob = useCallback(async () => {
    const plan = buildPlan()
    if (!plan) {
      return
    }

    try {
      setIsJobStarting(true)
      setFirmwareError('')
      setIsGroupStartModalOpen(false)
      const status = await invoke<FirmwareJobStatus>('start_firmware_update_job', { request: plan })
      setJobStatus(status)
      setGroupPasswordRecoveryResult(null)
      setIsGroupJobMinimized(false)
      setExpandedGroupIps(new Set())
    } catch (error) {
      setJobStatus(null)
      setFirmwareError(formatError(error))
    } finally {
      setIsJobStarting(false)
    }
  }, [buildPlan])

  const stopJob = useCallback(async () => {
    if (!jobStatus || isStoppingGroupJob || jobStatus.status === 'Cancelling') {
      return
    }

    setIsStoppingGroupJob(true)
    try {
      const status = await invoke<FirmwareJobStatus>('stop_firmware_update_job', { jobId: jobStatus.jobId })
      setJobStatus(status)
    } catch (error) {
      setFirmwareError(formatError(error))
    } finally {
      setIsStoppingGroupJob(false)
    }
  }, [isStoppingGroupJob, jobStatus])

  const forceRestoreGroupPasswords = useCallback(async () => {
    if (!jobStatus || isRestoringGroupPasswords) {
      return
    }

    setIsRestoringGroupPasswords(true)
    setGroupPasswordRecoveryResult(null)
    setFirmwareError('')
    try {
      const result = await invoke<FirmwarePasswordRecoveryBatchResult>('force_restore_firmware_job_passwords', {
        jobId: jobStatus.jobId,
      })
      setGroupPasswordRecoveryResult(result)
    } catch (error) {
      setFirmwareError(formatError(error))
    } finally {
      setIsRestoringGroupPasswords(false)
    }
  }, [isRestoringGroupPasswords, jobStatus])

  const groupJobId = jobStatus?.jobId || ''
  const groupJobState = jobStatus?.status || ''
  useEffect(() => {
    if (
      !groupJobId ||
      groupJobState === 'Completed' ||
      groupJobState === 'Failed' ||
      groupJobState === 'Cancelled' ||
      !isTauri()
    ) {
      return undefined
    }

    let cancelled = false
    const refreshStatus = async () => {
      try {
        const status = await invoke<FirmwareJobStatus>('get_firmware_job_status', { jobId: groupJobId })
        if (!cancelled) {
          setJobStatus(status)
        }
      } catch (error) {
        if (!cancelled) {
          setFirmwareError(formatError(error))
        }
      }
    }
    const interval = window.setInterval(() => void refreshStatus(), 1000)
    void refreshStatus()

    return () => {
      cancelled = true
      window.clearInterval(interval)
    }
  }, [groupJobId, groupJobState])

  const toggleReceiver = useCallback((ip: string) => {
    setSelectedIps((currentSelection) => {
      const next = new Set(currentSelection)
      if (next.has(ip)) {
        next.delete(ip)
      } else {
        next.add(ip)
      }
      return next
    })
  }, [])

  const selectAll = useCallback(() => {
    setSelectedIps(new Set(visibleFirmwareReceivers.map((receiver) => receiver.ip)))
  }, [visibleFirmwareReceivers])

  const clearSelection = useCallback(() => {
    setSelectedIps(new Set())
  }, [])

  const applyGroupFirmwarePackages = useCallback((selected: LocalFirmwarePackage[]) => {
      const next: Record<string, LocalFirmwarePackage> = {}
      const rejected: string[] = []
      selected.forEach((file) => {
        const model = firmwareModelFromPackageName(file.name)
        if (!model) {
          rejected.push(`${file.name}: receiver model is not present in the file name`)
          return
        }
        if (next[model]) {
          rejected.push(`${file.name}: more than one ${model.toUpperCase()} package was selected`)
          return
        }
        next[model] = file
      })

      setGroupLocalPackages(next)
      if (rejected.length) {
        setGroupPackageError(rejected.join('. '))
      }
  }, [])

  const pickGroupFirmwarePackages = useCallback(async () => {
    try {
      setIsGroupPackageLoading(true)
      setGroupPackageError('')
      const selected = await invoke<LocalFirmwarePackage[]>('pick_local_firmware_packages')
      if (selected.length) {
        applyGroupFirmwarePackages(selected)
      }
    } catch (error) {
      setGroupPackageError(formatError(error))
    } finally {
      setIsGroupPackageLoading(false)
    }
  }, [applyGroupFirmwarePackages])

  const handleGroupFirmwarePackageDrop = useCallback(
    async (event: DragEvent<HTMLDivElement>) => {
      event.preventDefault()
      setIsGroupPackageDragging(false)
      const paths = Array.from(event.dataTransfer.files)
        .map((file) => (file as File & { path?: string }).path || '')
        .filter(Boolean)
      if (!paths.length) {
        setGroupPackageError('Use Choose Files to select firmware packages from the native file picker.')
        return
      }

      try {
        setIsGroupPackageLoading(true)
        setGroupPackageError('')
        const selected = await Promise.all(
          paths.map((path) => invoke<LocalFirmwarePackage>('inspect_local_firmware_package', { path })),
        )
        applyGroupFirmwarePackages(selected)
      } catch (error) {
        setGroupPackageError(formatError(error))
      } finally {
        setIsGroupPackageLoading(false)
      }
    },
    [applyGroupFirmwarePackages],
  )

  const selectSingleFirmwareFile = useCallback((file: File | null) => {
    setSingleFileError('')

    if (!file) {
      setSingleFirmwareFile(null)
      setSingleFirmwareFilePath('')
      return
    }

    const lowerName = file.name.toLowerCase()
    if (!lowerName.endsWith('.img') && !lowerName.endsWith('.timg')) {
      setSingleFirmwareFile(null)
      setSingleFirmwareFilePath('')
      setSingleFileError('Only .img and .timg firmware packages are supported.')
      return
    }

    const filePath = (file as File & { path?: string }).path || ''
    setSingleFirmwareFile({
      name: file.name,
      size: file.size,
      path: filePath,
    })
    setSingleFirmwareFilePath(filePath)
    if (!filePath) {
      setSingleFileError('Local package is selected, but real update needs a readable local file path from the native picker.')
    }
  }, [])

  const applySelectedFirmwarePackage = useCallback(async (selected: LocalFirmwarePackage) => {
    setIsSingleFileLoading(true)
    setSingleFileError('')

    try {
      await new Promise((resolve) => window.setTimeout(resolve, 420))
      setSingleFirmwareFile({
        name: selected.name,
        size: selected.size,
        path: selected.path,
      })
      setSingleFirmwareFilePath(selected.path)
    } finally {
      setIsSingleFileLoading(false)
    }
  }, [])

  const pickSingleFirmwareFile = useCallback(async () => {
    try {
      setIsSingleFileLoading(true)
      setSingleFileError('')
      const selected = await invoke<LocalFirmwarePackage | null>('pick_local_firmware_package')
      if (!selected) {
        return
      }

      await applySelectedFirmwarePackage(selected)
    } catch (error) {
      setSingleFirmwareFile(null)
      setSingleFirmwareFilePath('')
      setSingleFileError(formatError(error))
    } finally {
      setIsSingleFileLoading(false)
    }
  }, [applySelectedFirmwarePackage])

  const handleFirmwareFileDrop = useCallback(
    async (event: DragEvent<HTMLDivElement>) => {
      event.preventDefault()
      setIsFirmwareFileDragging(false)

      const browserFile = event.dataTransfer.files.item(0)
      const browserPath = browserFile ? (browserFile as File & { path?: string }).path : ''
      if (browserPath) {
        try {
          const selected = await invoke<LocalFirmwarePackage>('inspect_local_firmware_package', { path: browserPath })
          await applySelectedFirmwarePackage(selected)
          return
        } catch (error) {
          setSingleFileError(formatError(error))
          return
        }
      }

      selectSingleFirmwareFile(browserFile)
    },
    [applySelectedFirmwarePackage, selectSingleFirmwareFile],
  )

  const applyMultipleFirmwarePackage = useCallback(async (selected: LocalFirmwarePackage) => {
    setIsMultipleFileLoading(true)
    setMultipleFileError('')
    await waitForUi(320)
    setMultipleFirmwareFile({
      name: selected.name,
      size: selected.size,
      path: selected.path,
    })
    setMultipleFirmwareFilePath(selected.path)
    setIsMultipleFileLoading(false)
  }, [])

  const pickMultipleFirmwareFile = useCallback(async () => {
    try {
      setIsMultipleFileLoading(true)
      setMultipleFileError('')
      const selected = await invoke<LocalFirmwarePackage | null>('pick_local_firmware_package')
      if (selected) {
        await applyMultipleFirmwarePackage(selected)
      }
    } catch (error) {
      setMultipleFirmwareFile(null)
      setMultipleFirmwareFilePath('')
      setMultipleFileError(formatError(error))
    } finally {
      setIsMultipleFileLoading(false)
    }
  }, [applyMultipleFirmwarePackage])

  const handleMultipleFirmwareFileDrop = useCallback(
    async (event: DragEvent<HTMLDivElement>) => {
      event.preventDefault()
      setIsMultipleFileDragging(false)
      const browserFile = event.dataTransfer.files.item(0)
      const browserPath = browserFile ? (browserFile as File & { path?: string }).path : ''
      if (!browserPath) {
        setMultipleFileError('Use the desktop file picker or drop a local .img / .timg package.')
        return
      }

      try {
        const selected = await invoke<LocalFirmwarePackage>('inspect_local_firmware_package', { path: browserPath })
        await applyMultipleFirmwarePackage(selected)
      } catch (error) {
        setMultipleFileError(formatError(error))
      }
    },
    [applyMultipleFirmwarePackage],
  )

  useEffect(() => {
    if (page !== 'FirmwareSingle' || !isTauri()) {
      return undefined
    }

    let unlisten: (() => void) | undefined
    let cancelled = false

    void getCurrentWindow()
      .onDragDropEvent(async (event: { payload: DragDropEvent }) => {
        if (cancelled || page !== 'FirmwareSingle' || singleSourceMode !== 'local') {
          return
        }

        if (event.payload.type === 'enter' || event.payload.type === 'over') {
          setIsFirmwareFileDragging(true)
          return
        }

        if (event.payload.type === 'leave') {
          setIsFirmwareFileDragging(false)
          return
        }

        if (event.payload.type === 'drop') {
          setIsFirmwareFileDragging(false)
          const [path] = event.payload.paths
          if (!path) {
            return
          }

          try {
            const selected = await invoke<LocalFirmwarePackage>('inspect_local_firmware_package', { path })
            await applySelectedFirmwarePackage(selected)
          } catch (error) {
            setSingleFirmwareFile(null)
            setSingleFirmwareFilePath('')
            setSingleFileError(formatError(error))
          }
        }
      })
      .then((dispose) => {
        unlisten = dispose
      })
      .catch((error) => {
        setSingleFileError(formatError(error))
      })

    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [applySelectedFirmwarePackage, page, singleSourceMode])

  useEffect(() => {
    if (page !== 'FirmwareMultiple' || !isTauri()) {
      return undefined
    }

    let unlisten: (() => void) | undefined
    let cancelled = false
    void getCurrentWindow()
      .onDragDropEvent(async (event: { payload: DragDropEvent }) => {
        if (cancelled || page !== 'FirmwareMultiple' || multipleSourceMode !== 'local') {
          return
        }
        if (event.payload.type === 'enter' || event.payload.type === 'over') {
          setIsMultipleFileDragging(true)
          return
        }
        if (event.payload.type === 'leave') {
          setIsMultipleFileDragging(false)
          return
        }
        if (event.payload.type === 'drop') {
          setIsMultipleFileDragging(false)
          const [path] = event.payload.paths
          if (!path) {
            return
          }
          try {
            const selected = await invoke<LocalFirmwarePackage>('inspect_local_firmware_package', { path })
            await applyMultipleFirmwarePackage(selected)
          } catch (error) {
            setMultipleFileError(formatError(error))
          }
        }
      })
      .then((dispose) => {
        unlisten = dispose
      })
      .catch((error) => setMultipleFileError(formatError(error)))

    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [applyMultipleFirmwarePackage, multipleSourceMode, page])

  const restoreSelectedReceiverPassword = useCallback(async () => {
    const receiverIp = singleReceiverIp.trim()
    if (!receiverIp) {
      return
    }

    try {
      setIsPasswordRestoreRunning(true)
      setPasswordRestoreResult(null)
      setFirmwareError('')
      const result = await invoke<PasswordRestoreResult>('restore_receiver_password', { receiverIp })
      setPasswordRestoreResult(result)
    } catch (error) {
      setFirmwareError(formatError(error))
    } finally {
      setIsPasswordRestoreRunning(false)
    }
  }, [singleReceiverIp])

  const upsertSingleUpdateJob = useCallback((status: FirmwareSingleUpdateJobStatus) => {
    setSingleUpdateJobs((current) => {
      const existing = current.findIndex((job) => job.jobId === status.jobId)
      if (existing === -1) {
        return [status, ...current].slice(0, 12)
      }

      const next = [...current]
      next[existing] = status
      return next
    })
  }, [])

  const openDowngradeReceiver = useCallback(async (ip: string) => {
    try {
      setFirmwareError('')
      await invoke('open_receiver', { ip })
    } catch (error) {
      setFirmwareError(formatError(error))
    }
  }, [])

  const resolveDowngrade = useCallback(
    async (jobId: string, approved: boolean, groupChild: boolean) => {
      setResolvingDowngradeJobIds((current) => new Set(current).add(jobId))
      try {
        setFirmwareError('')
        const status = await invoke<FirmwareSingleUpdateJobStatus>('resolve_firmware_downgrade', {
          jobId,
          approved,
        })
        if (groupChild) {
          setJobStatus((current) =>
            current
              ? {
                  ...current,
                  receivers: current.receivers.map((receiver) =>
                    receiver.childJobId === jobId
                      ? {
                          ...receiver,
                          downgradeDecision: approved,
                          message: status.message,
                        }
                      : receiver,
                  ),
                }
              : current,
          )
        } else {
          upsertSingleUpdateJob(status)
        }
      } catch (error) {
        setFirmwareError(formatError(error))
      } finally {
        setResolvingDowngradeJobIds((current) => {
          const next = new Set(current)
          next.delete(jobId)
          return next
        })
      }
    },
    [upsertSingleUpdateJob],
  )

  const runSingleLocalUpdate = useCallback(async () => {
    if (!singleFirmwareFile || !singleFirmwareFilePath) {
      return
    }

    try {
      setIsSingleUpdateStarting(true)
      setFirmwareError('')
      const status = await invoke<FirmwareSingleUpdateJobStatus>('start_single_local_firmware_update_job', {
        request: {
          receiverIp: singleReceiverIp.trim(),
          receiverModel: effectiveSingleReceiverModel || null,
          localFileName: singleFirmwareFile.name,
          localFilePath: singleFirmwareFilePath,
        },
      })
      upsertSingleUpdateJob(status)
      setActiveSingleUpdateJobId(status.jobId)
      setMinimizedSingleUpdateJobIds((current) => {
        const next = new Set(current)
        next.delete(status.jobId)
        return next
      })
      setSingleFirmwareFile(null)
      setSingleFirmwareFilePath('')
      if (firmwareFileInputRef.current) {
        firmwareFileInputRef.current.value = ''
      }
    } catch (error) {
      setFirmwareError(formatError(error))
    } finally {
      setIsSingleUpdateStarting(false)
    }
  }, [effectiveSingleReceiverModel, singleFirmwareFile, singleFirmwareFilePath, singleReceiverIp, upsertSingleUpdateJob])

  const runSingleReleaseUpdate = useCallback(async () => {
    try {
      setIsSingleUpdateStarting(true)
      setFirmwareError('')
      const status = await invoke<FirmwareSingleUpdateJobStatus>('start_single_release_firmware_update_job', {
        request: {
          mode: standaloneSelectedBuildMode,
          receiverIp: singleReceiverIp.trim(),
          receiverModel: effectiveSingleReceiverModel || null,
          firmwareVersion: standaloneSelectedReleaseId,
          firmwareSuffix: '',
          packageFile: singleCatalogPackage,
          sourceMode: 'release',
          localFileName: null,
        },
      })
      upsertSingleUpdateJob(status)
      setActiveSingleUpdateJobId(status.jobId)
      setMinimizedSingleUpdateJobIds((current) => {
        const next = new Set(current)
        next.delete(status.jobId)
        return next
      })
    } catch (error) {
      setFirmwareError(formatError(error))
    } finally {
      setIsSingleUpdateStarting(false)
    }
  }, [
    singleCatalogPackage,
    singleReceiverIp,
    effectiveSingleReceiverModel,
    standaloneSelectedBuildMode,
    standaloneSelectedReleaseId,
    upsertSingleUpdateJob,
  ])

  const updateMultipleBatchItem = useCallback(
    (batchId: string, ip: string, updater: (item: MultipleUpdateItem) => MultipleUpdateItem) => {
      setMultipleBatch((current) => {
        if (!current || current.batchId !== batchId) {
          return current
        }
        return {
          ...current,
          items: current.items.map((item) => (item.ip === ip ? updater(item) : item)),
        }
      })
    },
    [],
  )

  const runMultipleUpdate = useCallback(async () => {
    const sourceMode = multipleSourceMode
    const localFile = multipleFirmwareFile
    const localPath = multipleFirmwareFilePath
    const selected = multipleSelectedReceivers.map((receiver) => ({
      receiver,
      model: (multipleReceiverModels[receiver.ip] || '').trim().toLowerCase(),
    }))
    if (selected.length < 2) {
      return
    }

    const batchId = `multiple-fw-${Date.now()}`
    const targetLabel =
      sourceMode === 'local' && localFile
        ? localFile.name
        : `${standaloneSelectedBuildMode} · ${standaloneReleaseInspection?.buildName || standaloneSelectedBuildId}`
    const initialBatch: MultipleUpdateBatch = {
      batchId,
      status: 'Running',
      sourceMode,
      firmwareVersion: sourceMode === 'release' ? standaloneSelectedReleaseId : '',
      targetLabel,
      startedAt: String(Math.floor(Date.now() / 1000)),
      items: selected.map(({ receiver, model }) => ({
        ip: receiver.ip,
        receiverName: receiver.receiverName,
        model,
        status: 'Queued',
        progress: 0,
        phase: 'Queued',
        message: 'Waiting for an available update slot.',
        currentFirmware: receiver.firmwareVersion,
        targetFile:
          sourceMode === 'local' && localFile ? localFile.name : multipleCatalogPackages[model] || 'Package pending',
        jobId: null,
      })),
    }

    setIsMultipleUpdateStarting(true)
    setFirmwareError('')
    setMultipleBatch(initialBatch)
    setIsMultipleBatchMinimized(false)
    setExpandedMultipleIps(new Set())

    if (sourceMode === 'local') {
      setMultipleFirmwareFile(null)
      setMultipleFirmwareFilePath('')
      if (multipleFirmwareFileInputRef.current) {
        multipleFirmwareFileInputRef.current.value = ''
      }
    }

    const outcomes = new Map<string, 'Completed' | 'Failed'>()
    let cursor = 0
    const workerCount = Math.max(1, Math.min(3 - activeSingleUpdateJobs.length, selected.length))

    const runReceiver = async (receiver: Receiver, model: string) => {
      updateMultipleBatchItem(batchId, receiver.ip, (item) => ({
        ...item,
        status: 'Starting',
        message: 'Preparing receiver update job.',
      }))

      let initialStatus: FirmwareSingleUpdateJobStatus | null = null
      while (!initialStatus) {
        try {
          initialStatus =
            sourceMode === 'release'
              ? await invoke<FirmwareSingleUpdateJobStatus>('start_single_release_firmware_update_job', {
                  request: {
                    mode: standaloneSelectedBuildMode,
                    batchId,
                    receiverIp: receiver.ip,
                    receiverModel: model,
                    firmwareVersion: standaloneSelectedReleaseId,
                    firmwareSuffix: '',
                    packageFile: multipleCatalogPackages[model] || null,
                    sourceMode: 'release',
                    localFileName: null,
                  },
                })
              : await invoke<FirmwareSingleUpdateJobStatus>('start_single_local_firmware_update_job', {
                  request: {
                    batchId,
                    receiverIp: receiver.ip,
                    receiverModel: model,
                    localFileName: localFile?.name || '',
                    localFilePath: localPath,
                  },
                })
        } catch (error) {
          const message = formatError(error)
          if (message.includes('Maximum 3 single receiver updates')) {
            await waitForUi(1800)
            continue
          }
          updateMultipleBatchItem(batchId, receiver.ip, (item) => ({
            ...item,
            status: 'Failed',
            phase: 'Failed',
            message,
          }))
          outcomes.set(receiver.ip, 'Failed')
          return
        }
      }

      updateMultipleBatchItem(batchId, receiver.ip, (item) => ({
        ...item,
        status: 'Running',
        jobId: initialStatus?.jobId || null,
        targetFile: initialStatus?.targetFile || item.targetFile,
        phase: initialStatus?.phase || 'Queued',
        message: initialStatus?.message || item.message,
      }))

      let status = initialStatus
      while (status.status !== 'Completed' && status.status !== 'Failed') {
        await waitForUi(1500)
        try {
          status = await invoke<FirmwareSingleUpdateJobStatus>('get_single_firmware_update_job_status', {
            jobId: status.jobId,
          })
          updateMultipleBatchItem(batchId, receiver.ip, (item) => ({
            ...item,
            status: status.status === 'Failed' ? 'Failed' : status.status === 'Completed' ? 'Completed' : 'Running',
            progress: getFirmwareJobProgress(status),
            phase: getFirmwareJobDisplayState(status),
            message: status.message,
            currentFirmware: status.firmwareVersion || item.currentFirmware,
            targetFile: status.targetFile,
          }))
        } catch (error) {
          const message = formatError(error)
          updateMultipleBatchItem(batchId, receiver.ip, (item) => ({
            ...item,
            status: 'Failed',
            phase: 'Status Error',
            message,
          }))
          outcomes.set(receiver.ip, 'Failed')
          return
        }
      }

      outcomes.set(receiver.ip, status.status === 'Completed' ? 'Completed' : 'Failed')
    }

    const worker = async () => {
      while (cursor < selected.length) {
        const index = cursor
        cursor += 1
        const item = selected[index]
        await runReceiver(item.receiver, item.model)
      }
    }

    await Promise.all(Array.from({ length: workerCount }, () => worker()))
    const failed = Array.from(outcomes.values()).some((status) => status === 'Failed')
    setMultipleBatch((current) =>
      current?.batchId === batchId
        ? {
            ...current,
            status: failed ? 'Failed' : 'Completed',
          }
        : current,
    )
    setIsMultipleUpdateStarting(false)
  }, [
    activeSingleUpdateJobs.length,
    multipleFirmwareFile,
    multipleFirmwareFilePath,
    multipleCatalogPackages,
    multipleReceiverModels,
    multipleSelectedReceivers,
    multipleSourceMode,
    standaloneReleaseInspection?.buildName,
    standaloneSelectedBuildId,
    standaloneSelectedBuildMode,
    standaloneSelectedReleaseId,
    updateMultipleBatchItem,
  ])

  useEffect(() => {
    const runningJobIds = singleUpdateJobs
      .filter((job) => job.status !== 'Completed' && job.status !== 'Failed' && job.status !== 'Cancelled')
      .map((job) => job.jobId)

    if (runningJobIds.length === 0) {
      return undefined
    }

    let cancelled = false
    const poll = async () => {
      await Promise.all(
        runningJobIds.map(async (jobId) => {
          try {
            const status = await invoke<FirmwareSingleUpdateJobStatus>('get_single_firmware_update_job_status', {
              jobId,
            })
            if (!cancelled) {
              upsertSingleUpdateJob(status)
            }
          } catch (error) {
            if (!cancelled) {
              setFirmwareError(formatError(error))
            }
          }
        }),
      )
    }

    const interval = window.setInterval(() => {
      void poll()
    }, 1500)
    void poll()

    return () => {
      cancelled = true
      window.clearInterval(interval)
    }
  }, [singleUpdateJobs, upsertSingleUpdateJob])

  const activeSingleUpdateJob = activeSingleUpdateJobId
    ? singleUpdateJobs.find((job) => job.jobId === activeSingleUpdateJobId) ?? null
    : null
  const singleUpdateOrbJobs = singleUpdateJobs.filter(
    (job) => job.jobId !== activeSingleUpdateJob?.jobId || minimizedSingleUpdateJobIds.has(job.jobId),
  )
  const getSingleUpdateProgress = (job: FirmwareSingleUpdateJobStatus) =>
    job.status === 'Completed' ? 100 : Math.max(0, Math.min(100, job.updatePercent ?? 0))
  const canDismissSingleUpdateStatus = (job: FirmwareSingleUpdateJobStatus) =>
    job.status === 'Completed' || job.status === 'Failed' || job.status === 'Cancelled'
  const dismissSingleUpdateStatus = (jobId: string) => {
    setSingleUpdateJobs((current) => current.filter((job) => job.jobId !== jobId))
    setActiveSingleUpdateJobId((current) => (current === jobId ? null : current))
    setMinimizedSingleUpdateJobIds((current) => {
      const next = new Set(current)
      next.delete(jobId)
      return next
    })
  }
  const multipleOverallProgress = multipleBatch
    ? multipleBatch.items.reduce(
        (total, item) => total + (item.status === 'Completed' || item.status === 'Failed' ? 100 : item.progress),
        0,
      ) / Math.max(1, multipleBatch.items.length)
    : 0
  const multipleCompletedCount =
    multipleBatch?.items.filter((item) => item.status === 'Completed').length || 0
  const multipleFailedCount = multipleBatch?.items.filter((item) => item.status === 'Failed').length || 0
  const canDismissMultipleBatch = multipleBatch?.status === 'Completed' || multipleBatch?.status === 'Failed'
  const groupCompletedCount = jobStatus?.receivers.filter((receiver) => receiver.status === 'Completed').length || 0
  const groupFailedCount = jobStatus?.receivers.filter((receiver) => receiver.status === 'Failed').length || 0
  const groupCancelledCount = jobStatus?.receivers.filter((receiver) => receiver.status === 'Cancelled').length || 0
  const canDismissGroupJob =
    jobStatus?.status === 'Completed' || jobStatus?.status === 'Failed' || jobStatus?.status === 'Cancelled'

  useEffect(() => {
    singleUpdateJobs.forEach((job) => {
      const exactProgress = job.status === 'Completed' ? 100 : Math.max(0, Math.min(100, job.updatePercent ?? 0))
      const reportedProgress =
        job.status === 'Running' ? Math.floor(exactProgress / 10) * 10 : Math.round(exactProgress)
      const phase = getFirmwareJobDisplayState(job)
      const signature = `${job.status}|${phase}|${reportedProgress}`

      if (reportedSingleActivityRef.current.get(job.jobId) === signature) {
        return
      }

      reportedSingleActivityRef.current.set(job.jobId, signature)
      if (job.status === 'Completed') {
        onActivity(`Firmware update completed for ${job.ip}: ${job.targetFile}`)
      } else if (job.status === 'Cancelled') {
        onActivity(`Firmware update cancelled for ${job.ip}: receiver reservation released`)
      } else if (job.status === 'Failed') {
        onActivity(`Firmware update failed for ${job.ip}: ${job.message}`)
      } else {
        onActivity(`Firmware update ${job.ip}: ${phase} - ${reportedProgress}%`)
      }
    })
  }, [onActivity, singleUpdateJobs])

  useEffect(() => {
    if (!multipleBatch) {
      reportedMultipleActivityRef.current = ''
      return
    }

    const exactProgress =
      multipleBatch.items.reduce(
        (total, item) => total + (item.status === 'Completed' || item.status === 'Failed' ? 100 : item.progress),
        0,
      ) / Math.max(1, multipleBatch.items.length)
    const reportedProgress =
      multipleBatch.status === 'Running' ? Math.floor(exactProgress / 10) * 10 : Math.round(exactProgress)
    const completed = multipleBatch.items.filter((item) => item.status === 'Completed').length
    const failed = multipleBatch.items.filter((item) => item.status === 'Failed').length
    const signature = `${multipleBatch.batchId}|${multipleBatch.status}|${reportedProgress}|${completed}|${failed}`

    if (reportedMultipleActivityRef.current === signature) {
      return
    }

    reportedMultipleActivityRef.current = signature
    onActivity(
      `Multiple firmware update: ${reportedProgress}% - ${completed}/${multipleBatch.items.length} completed${
        failed ? `, ${failed} failed` : ''
      }`,
    )
  }, [multipleBatch, onActivity])

  useEffect(() => {
    if (!jobStatus) {
      reportedGroupActivityRef.current = ''
      return
    }

    const reportedProgress =
      jobStatus.status === 'Running' || jobStatus.status === 'Cancelling'
        ? Math.floor(jobStatus.progress / 10) * 10
        : Math.round(jobStatus.progress)
    const signature = `${jobStatus.jobId}|${jobStatus.status}|${reportedProgress}|${groupCompletedCount}|${groupCancelledCount}|${groupFailedCount}`
    if (reportedGroupActivityRef.current === signature) {
      return
    }

    reportedGroupActivityRef.current = signature
    onActivity(
      `${jobStatus.groupName} firmware update: ${reportedProgress}% - ${groupCompletedCount}/${jobStatus.receivers.length} completed${
        groupCancelledCount ? `, ${groupCancelledCount} cancelled` : ''
      }${
        groupFailedCount ? `, ${groupFailedCount} failed` : ''
      }`,
    )
  }, [groupCancelledCount, groupCompletedCount, groupFailedCount, jobStatus, onActivity])

  const renderStandaloneReleaseCatalog = (disabled: boolean) => (
    <div className="standalone-release-catalog">
      <label className="firmware-field">
        <span>Release</span>
        <div className="release-select-row">
          <select
            value={standaloneSelectedReleaseId}
            disabled={disabled || isReleaseCatalogLoading || !standaloneReleaseOptions.length}
            onChange={(event) => {
              setStandaloneSelectedReleaseId(event.target.value)
              setStandaloneBuildOptions([])
              setStandaloneSelectedBuildId('')
              setStandaloneSelectedBuildMode('')
              setStandaloneReleaseInspection(null)
              setStandaloneCatalogError('')
            }}
          >
            <option value="">
              {isReleaseCatalogLoading ? 'Loading releases...' : 'Select an Alpha or Beta release'}
            </option>
            {standaloneReleaseOptions.map((release) => (
              <option value={release.id} key={release.id}>
                {release.name}
              </option>
            ))}
          </select>
          <button
            className="icon-button release-refresh-button"
            type="button"
            title="Refresh release catalog"
            aria-label="Refresh release catalog"
            disabled={disabled || isReleaseCatalogLoading}
            onClick={() => void loadReleaseCatalog()}
          >
            <RefreshCw className={isReleaseCatalogLoading ? 'spin' : ''} size={16} />
          </button>
        </div>
      </label>

      <label className="firmware-field">
        <span>Build</span>
        <select
          value={standaloneSelectedBuildMode ? `${standaloneSelectedBuildMode}::${standaloneSelectedBuildId}` : ''}
          disabled={disabled || !standaloneSelectedReleaseId || isStandaloneBuildLoading}
          onChange={(event) => {
            const [mode, ...idParts] = event.target.value.split('::')
            setStandaloneSelectedBuildMode(mode === 'Alpha' || mode === 'Beta' ? mode : '')
            setStandaloneSelectedBuildId(idParts.join('::'))
            setStandaloneReleaseInspection(null)
            setStandaloneCatalogError('')
          }}
        >
          <option value="">
            {isStandaloneBuildLoading
              ? 'Loading builds...'
              : standaloneSelectedReleaseId
                ? 'Select a server build'
                : 'Select a release first'}
          </option>
          {standaloneBuildOptions.map((build) => (
            <option value={`${build.mode}::${build.id}`} key={`${build.mode}:${build.id}`}>
              [{build.mode}] {build.name}
            </option>
          ))}
        </select>
      </label>

      <div className="standalone-package-summary">
        <div>
          <span>Model Packages</span>
          <small>Resolved directly from the selected server build</small>
        </div>
        {isStandaloneInspectionLoading ? (
          <span className="catalog-loading-label">
            <Loader2 className="spin" size={14} /> Checking packages
          </span>
        ) : null}
        {!isStandaloneInspectionLoading && standaloneInspectionModels.length > 0 ? (
          <div className="release-model-matches">
            {standaloneInspectionModels.map((model) => {
              const match = standaloneReleaseMatchesByModel.get(model)
              const ready = match?.status === 'Matched' && Boolean(match.selectedFile)
              return (
                <div className={`release-model-match ${ready ? 'matched' : 'missing'}`} key={model}>
                  <strong>{model.toUpperCase()}</strong>
                  <span>{ready ? 'Ready' : 'Missing'}</span>
                  <small title={match?.selectedFile || 'Package not found'}>
                    {match?.selectedFile || 'Select receiver model or another build'}
                  </small>
                </div>
              )
            })}
          </div>
        ) : (
          <p className="catalog-empty-copy">
            {standaloneSelectedBuildId
              ? 'Select receiver model information to resolve the package.'
              : 'Select a release and build to verify firmware packages.'}
          </p>
        )}
      </div>

      {standaloneCatalogError ? (
        <div className="firmware-catalog-alert" role="alert">
          <AlertTriangle size={16} />
          <span>{standaloneCatalogError}</span>
        </div>
      ) : null}
    </div>
  )

  return (
    <section className="surface firmware-surface">
      <header className="firmware-header">
        <div className="firmware-title">
          <div className="firmware-icon">
            <Icon size={22} />
          </div>
          <div>
            <h2>{current.title}</h2>
            <p>{current.subtitle}</p>
          </div>
        </div>
        <span className="status-pill pending">
          {isGroupUpdate
            ? 'Group update'
            : page === 'FirmwareSingle'
              ? 'Up to 3 updates'
              : page === 'FirmwareMultiple'
                ? 'Managed session'
                : 'Group setup'}
        </span>
      </header>

      {isGroupUpdate ? (
        <div className="firmware-page-content firmware-group-page firmware-operation-page group-operation-page firmware-unified-workspace">
          <section className="firmware-plan-toolbar firmware-settings-pane firmware-config-panel">
            <div className="firmware-panel-header">
              <div>
                <h3>Firmware Plan</h3>
                <p>Confirm the target release and choose the receivers included in this update.</p>
              </div>
              <div className="firmware-panel-actions">
                <button
                  className="secondary-button multiple-select-visible"
                  disabled={hasActiveGroupJob}
                  onClick={() => {
                    setFirmwareError('')
                    setReceiverPendingModelEdit(null)
                    setNewReceiverIp('')
                    setNewReceiverModel('')
                    setIsAddReceiverModalOpen(true)
                  }}
                >
                  <Plus size={16} />
                  Add Config Receiver
                </button>
                <button
                  className="secondary-button"
                  onClick={() => void loadFirmwareReceivers()}
                  disabled={hasActiveGroupJob || isLoadingReceivers}
                >
                  <RefreshCw className={isLoadingReceivers ? 'spin' : ''} size={16} />
                  Reload Group
                </button>
              </div>
            </div>

            <div className="group-release-mode-row">
              <div className="source-mode-switch" aria-label="Group firmware target source">
                <button
                  type="button"
                  className={groupReleaseMode === 'catalog' ? 'active' : ''}
                  onClick={() => setGroupReleaseMode('catalog')}
                  disabled={hasActiveGroupJob || (!releaseCatalog && isReleaseCatalogLoading)}
                >
                  <Server size={15} />
                  Release Catalog
                </button>
                <button
                  type="button"
                  className={groupReleaseMode === 'local' ? 'active' : ''}
                  onClick={() => {
                    setGroupReleaseMode('local')
                    setGroupPackageError('')
                  }}
                  disabled={hasActiveGroupJob}
                >
                  <UploadCloud size={15} />
                  Local Packages
                </button>
              </div>
            </div>

            {groupReleaseMode === 'catalog' ? (
              <div className="firmware-form-grid group-plan-fields catalog-plan-fields">
                <label className="firmware-field group-release-selector">
                  <span className="firmware-field-label-row">
                    <span>Release</span>
                    <small>Active server releases · Manual selection required</small>
                  </span>
                  <div className="release-select-row">
                    <select
                      value={activeSelectedReleaseId}
                      onChange={(event) => {
                        setSelectedReleaseId(event.target.value)
                        setSelectedReleaseMode(event.target.value ? current.mode || '' : '')
                        setBuildCatalog(null)
                        setSelectedBuildId('')
                        setReleaseInspection(null)
                        setReleaseCatalogError('')
                        setReleaseCatalogErrorMode('')
                      }}
                      disabled={hasActiveGroupJob || isReleaseCatalogLoading || !visibleReleaseOptions.length}
                    >
                      <option value="">
                        {isReleaseCatalogLoading ? 'Loading releases...' : 'Select a server release'}
                      </option>
                      {visibleReleaseOptions.map((release) => (
                        <option key={release.id} value={release.id}>
                          {release.name}
                        </option>
                      ))}
                    </select>
                    <button
                      className="icon-button release-refresh-button"
                      type="button"
                      title="Refresh release catalog"
                      aria-label="Refresh release catalog"
                      onClick={() => void loadReleaseCatalog()}
                      disabled={hasActiveGroupJob || isReleaseCatalogLoading}
                    >
                      <RefreshCw className={isReleaseCatalogLoading ? 'spin' : ''} size={16} />
                    </button>
                  </div>
                </label>

                <label className="firmware-field group-build-selector">
                  <span className="firmware-field-label-row">
                    <span>Build</span>
                    <small>{current.mode} builds only · Manual selection required</small>
                  </span>
                  <select
                    value={activeSelectedBuildId}
                    onChange={(event) => {
                      setSelectedBuildId(event.target.value)
                      setReleaseInspection(null)
                    }}
                    disabled={
                      hasActiveGroupJob ||
                      !activeSelectedReleaseId ||
                      isCurrentBuildCatalogLoading ||
                      !buildCatalog?.builds.length
                    }
                  >
                    <option value="">
                      {isCurrentBuildCatalogLoading
                        ? 'Loading builds...'
                        : activeSelectedReleaseId
                          ? 'Select a server build'
                          : 'Select a release first'}
                    </option>
                    {buildCatalog?.builds.map((build) => (
                      <option key={build.id} value={build.id}>
                        {build.name}
                      </option>
                    ))}
                  </select>
                </label>

                <div className="release-package-map" aria-live="polite">
                  <div className="release-package-map-header">
                    <div>
                      <span>Model Packages</span>
                      <small>Verified for {current.mode} receiver models</small>
                    </div>
                    {currentReleaseInspection ? (
                      <span className={`release-package-summary ${configuredPackagesReady ? 'ready' : 'warning'}`}>
                        {configuredPackagesReady
                          ? `${configuredGroupModels.length} models ready`
                          : `${readyConfiguredModelCount}/${configuredGroupModels.length} models ready`}
                      </span>
                    ) : null}
                  </div>
                  <div className="release-model-matches">
                    {isCurrentBuildCatalogLoading || isCurrentReleaseInspectionLoading ? (
                      <span className="release-map-placeholder">
                        <Loader2 className="spin" size={15} />
                        {isCurrentBuildCatalogLoading
                          ? `Loading ${current.mode} builds from the selected release`
                          : 'Reading package names from the selected build'}
                      </span>
                    ) : currentReleaseInspection ? (
                      configuredGroupModels.map((model) => {
                        const match = releaseMatchesByModel.get(model)
                        const matchStatus = match?.status.toLowerCase() || 'missing'
                        const details =
                          match?.status === 'Matched'
                            ? match.selectedFile || ''
                            : match?.status === 'Ambiguous'
                              ? `${match.candidates.length} matching files`
                              : 'Package not found'
                        return (
                          <span
                            className={`release-model-match ${matchStatus}`}
                            key={model}
                            title={match?.candidates.join('\n') || details}
                          >
                            <span className="release-model-match-heading">
                              <strong>{model}</strong>
                              <span>{match?.status === 'Matched' ? 'Ready' : match?.status || 'Missing'}</span>
                            </span>
                            <small>{details}</small>
                          </span>
                        )
                      })
                    ) : (
                      <span className="release-map-placeholder">
                        Select a release and build to verify packages for this group.
                      </span>
                    )}
                  </div>
                </div>

                <label className="firmware-toggle">
                  <input
                    type="checkbox"
                    checked={parallel}
                    onChange={(event) => setParallel(event.target.checked)}
                    disabled={hasActiveGroupJob}
                  />
                  <span>Parallel mode</span>
                </label>
              </div>
            ) : (
              <div className="firmware-form-grid group-plan-fields group-local-package-fields">
                <div
                  className={`group-package-picker ${isGroupPackageDragging ? 'dragging' : ''}`}
                  onDragOver={(event) => {
                    event.preventDefault()
                    setIsGroupPackageDragging(true)
                  }}
                  onDragLeave={() => setIsGroupPackageDragging(false)}
                  onDrop={handleGroupFirmwarePackageDrop}
                >
                  <div>
                    <strong>Firmware Packages</strong>
                    <span>
                      Select one package for every receiver model in the current selection. File names must contain
                      the model, for example "argon" or "silk".
                    </span>
                  </div>
                  <button
                    className="secondary-button"
                    type="button"
                    onClick={() => void pickGroupFirmwarePackages()}
                    disabled={hasActiveGroupJob || isGroupPackageLoading}
                  >
                    {isGroupPackageLoading ? <Loader2 className="spin" size={16} /> : <UploadCloud size={16} />}
                    Choose Files
                  </button>
                </div>
                <div className="release-package-map group-local-package-map" aria-live="polite">
                  <div className="release-package-map-header">
                    <div>
                      <span>Required Model Packages</span>
                      <small>Calculated from the selected receivers</small>
                    </div>
                    <span className={`release-package-summary ${groupLocalPackagesReady ? 'ready' : 'warning'}`}>
                      {selectedGroupModels.filter((model) => Boolean(groupLocalPackages[model])).length}/
                      {selectedGroupModels.length} models ready
                    </span>
                  </div>
                  <div className="release-model-matches">
                    {selectedGroupModels.length ? (
                      selectedGroupModels.map((model) => {
                        const selectedPackage = groupLocalPackages[model]
                        const validModel = RECEIVER_MODEL_OPTIONS.includes(model)
                        return (
                          <span
                            className={`release-model-match ${selectedPackage && validModel ? 'matched' : 'missing'}`}
                            key={model}
                            title={selectedPackage?.path || undefined}
                          >
                            <span className="release-model-match-heading">
                              <strong>{model}</strong>
                              <span>{selectedPackage && validModel ? 'Ready' : 'Missing'}</span>
                            </span>
                            <small>
                              {validModel
                                ? selectedPackage?.name || 'Select a package containing this model name'
                                : 'Assign the receiver model before starting'}
                            </small>
                          </span>
                        )
                      })
                    ) : (
                      <span className="release-map-placeholder">Select at least one receiver to calculate packages.</span>
                    )}
                  </div>
                </div>
                <label className="firmware-toggle">
                  <input
                    type="checkbox"
                    checked={parallel}
                    onChange={(event) => setParallel(event.target.checked)}
                    disabled={hasActiveGroupJob}
                  />
                  <span>Parallel mode</span>
                </label>
                {groupPackageError ? <p className="credentials-error group-package-error">{groupPackageError}</p> : null}
              </div>
            )}

            {visibleReleaseCatalogError ? (
              <div className="firmware-catalog-alert" role="alert">
                <AlertTriangle size={16} />
                <span>{visibleReleaseCatalogError}</span>
              </div>
            ) : null}
          </section>

          <section className="firmware-list-panel firmware-receiver-pane firmware-receiver-list-panel">
            <div className="firmware-selection-toolbar">
              <div className="firmware-list-heading">
                <h3>Receivers</h3>
                <span>{selectedIps.size} of {visibleFirmwareReceivers.length} selected</span>
              </div>
              <div>
                <button
                  className="secondary-button"
                  onClick={selectAll}
                  disabled={hasActiveGroupJob || !visibleFirmwareReceivers.length}
                >
                  Select All
                </button>
                <button
                  className="secondary-button"
                  onClick={clearSelection}
                  disabled={hasActiveGroupJob || !selectedIps.size}
                >
                  Clear
                </button>
              </div>
            </div>

            <div className="firmware-device-list group-firmware-device-list">
              {visibleFirmwareReceivers.map((receiver) => {
                const jobReceiver = jobStatus?.receivers.find((item) => item.ip === receiver.ip)
                const receiverStatus = receiverStatusByIp.get(receiver.ip)
                const receiverName = receiverStatus?.receiverName || receiver.ip
                const currentFirmware = jobReceiver?.currentFirmware || receiverStatus?.firmwareVersion || null
                const uptimeSeconds = receiverStatus?.uptimeSeconds ?? null
                const updateStatus = jobReceiver?.status || 'Pending'
                const isOffline = receiverStatus?.status === 'offline'
                const displayStatus: ReceiverStatus = isOffline
                  ? 'offline'
                  : receiverStatus?.status || 'pending'
                return (
                  <FirmwareDeviceCard
                    key={receiver.ip}
                    className="group-firmware-device-card"
                    selected={selectedIps.has(receiver.ip)}
                    name={receiverName}
                    ip={receiver.ip}
                    groupLabel={receiver.groupName}
                    model={receiver.model}
                    status={displayStatus}
                    firmwareVersion={currentFirmware}
                    uptimeSeconds={uptimeSeconds}
                    fixType={receiverStatus?.fixType}
                    celsius={receiverStatus?.celsius}
                    svSolved={receiverStatus?.svSolved}
                    credentialsReady={receiver.hasCredentials}
                    leading={
                      <label title={`Select ${receiver.ip}`}>
                        <input
                          type="checkbox"
                          checked={selectedIps.has(receiver.ip)}
                          onChange={() => toggleReceiver(receiver.ip)}
                          disabled={hasActiveGroupJob}
                          aria-label={`Select ${receiver.ip}`}
                        />
                      </label>
                    }
                    footer={
                      jobReceiver ? (
                        <div className="group-receiver-progress">
                          <div>
                            <span>
                              {jobReceiver.phase === 'Preflight'
                                ? 'Receiver Check'
                                : formatWorkflowState(jobReceiver.phase)}
                            </span>
                            <strong>{Math.round(jobReceiver.progress)}%</strong>
                          </div>
                          <div className="group-receiver-progress-track" aria-hidden="true">
                            <span style={{ width: `${Math.max(0, Math.min(100, jobReceiver.progress))}%` }} />
                          </div>
                        </div>
                      ) : null
                    }
                    actions={
                      <>
                      <span className={`preflight-pill ${updateStatus.toLowerCase()}`}>
                        {updateStatus}
                      </span>
                      <button
                        className="firmware-edit-button"
                        type="button"
                        title={`Edit firmware model for ${receiver.ip}`}
                        aria-label={`Edit firmware model for ${receiver.ip}`}
                        onClick={(event) => {
                          event.preventDefault()
                          event.stopPropagation()
                          setFirmwareError('')
                          setReceiverPendingModelEdit(receiver)
                          setNewReceiverIp(receiver.ip)
                          setNewReceiverModel(receiver.model.toLowerCase() === 'unknown' ? '' : receiver.model)
                          setIsAddReceiverModalOpen(true)
                        }}
                        disabled={hasActiveGroupJob || isSavingReceiver}
                      >
                        <Pencil size={15} />
                      </button>
                      <button
                        className="firmware-remove-button"
                        title={`Remove ${receiver.ip} from firmware config`}
                        onClick={(event) => {
                          event.preventDefault()
                          event.stopPropagation()
                          setFirmwareError('')
                          setRemovePassword('')
                          setShowRemovePassword(false)
                          setReceiverPendingRemoval(receiver)
                        }}
                        disabled={hasActiveGroupJob || isSavingReceiver}
                      >
                        <X size={16} />
                      </button>
                      </>
                    }
                  />
                )
              })}

              {!visibleFirmwareReceivers.length && (
                <div className="firmware-empty">
                  <Radio size={34} />
                  <strong>No receivers loaded</strong>
                  <p>Reload this group after the receiver configuration becomes available.</p>
                </div>
              )}
            </div>
          </section>

          <footer className="firmware-action-dock firmware-settings-actions">
            <div className="firmware-operation-summary">
              <div className="firmware-operation-copy">
                <strong>{selectedIps.size} receivers selected</strong>
                <span>
                  {jobStatus
                    ? `Group progress ${Math.round(jobStatus.progress)}%. Receiver password recovery remains active.`
                    : 'Confirm the mapped packages before starting this controlled group update.'}
                </span>
              </div>
              {jobStatus ? (
                <div className="firmware-job-inline">
                  <span className={`preflight-pill ${jobStatus.status.toLowerCase()}`}>{jobStatus.status}</span>
                  <span>
                    {jobStatus.receivers.filter((receiver) => receiver.status === 'Completed').length} completed,{' '}
                    {jobStatus.receivers.filter((receiver) => receiver.status === 'Failed').length} failed
                  </span>
                </div>
              ) : null}
              {firmwareError && (
                <div className="firmware-inline-alert" role="alert">
                  <AlertTriangle size={17} />
                  <span>{firmwareError}</span>
                </div>
              )}
            </div>

            <div className="firmware-dock-actions">
              {hasActiveGroupJob && (
                <>
                  <button
                    className="secondary-button group-password-recovery-button"
                    disabled={isRestoringGroupPasswords}
                    onClick={() => void forceRestoreGroupPasswords()}
                  >
                    {isRestoringGroupPasswords ? <Loader2 className="spin" size={16} /> : <KeyRound size={16} />}
                    Force Restore Passwords
                  </button>
                  <button
                    className="secondary-button"
                    disabled={isStoppingGroupJob || jobStatus?.status === 'Cancelling'}
                    onClick={() => void stopJob()}
                  >
                    {isStoppingGroupJob || jobStatus?.status === 'Cancelling' ? (
                      <Loader2 className="spin" size={16} />
                    ) : (
                      <X size={16} />
                    )}
                    {jobStatus?.status === 'Cancelling' ? 'Cancelling Safely' : 'Cancel Update'}
                  </button>
                </>
              )}
              <button
                className="primary-button"
                onClick={() => setIsGroupStartModalOpen(true)}
                disabled={!canRunPlan || isJobStarting}
              >
                {isJobStarting ? <Loader2 className="spin" size={16} /> : <UploadCloud size={16} />}
                Start Group Update
              </button>
            </div>
          </footer>
        </div>
      ) : page === 'FirmwareMultiple' ? (
        <div className="firmware-page-scroll multiple-update-body firmware-operation-page">
          <section className="multiple-update-workspace firmware-operation-layout firmware-unified-workspace">
            <div className="multiple-selection-panel firmware-receiver-pane firmware-receiver-list-panel">
              <div className="multiple-section-header">
                <div>
                  <h3>Receiver Selection</h3>
                  <p>Select at least two online receivers and confirm the hardware model for each device.</p>
                </div>
                <span className="multiple-selection-count">{multipleSelectedIps.size} selected</span>
              </div>

              <div className="multiple-selection-tools">
                <label className="history-search">
                  <Search size={17} />
                  <input
                    value={multipleQuery}
                    onChange={(event) => setMultipleQuery(event.target.value)}
                    placeholder="Search IP, receiver, firmware or model"
                  />
                </label>
                <button
                  className="secondary-button"
                  type="button"
                  onClick={() => {
                    const visibleOnlineIps = multipleReceiverOptions
                      .filter((receiver) => receiver.status === 'online')
                      .map((receiver) => receiver.ip)
                    const allSelected =
                      visibleOnlineIps.length > 0 && visibleOnlineIps.every((ip) => multipleSelectedIps.has(ip))
                    if (!allSelected && multipleSourceMode === 'local' && multipleSharedLocalModel) {
                      setMultipleReceiverModels((current) => {
                        const next = { ...current }
                        visibleOnlineIps.forEach((ip) => {
                          next[ip] = multipleSharedLocalModel
                        })
                        return next
                      })
                    }
                    setMultipleSelectedIps((current) => {
                      const next = new Set(current)
                      visibleOnlineIps.forEach((ip) => (allSelected ? next.delete(ip) : next.add(ip)))
                      return next
                    })
                  }}
                >
                  {multipleReceiverOptions.some((receiver) => receiver.status === 'online') &&
                  multipleReceiverOptions
                    .filter((receiver) => receiver.status === 'online')
                    .every((receiver) => multipleSelectedIps.has(receiver.ip))
                    ? 'Clear Visible'
                    : 'Select Visible'}
                </button>
              </div>

              <div className="multiple-receiver-list">
                {multipleReceiverOptions.map((receiver) => {
                  const selected = multipleSelectedIps.has(receiver.ip)
                  const offline = receiver.status !== 'online'
                  return (
                    <article
                      className={`update-receiver-card multiple-update-receiver-card ${selected ? 'selected' : ''} ${
                        offline ? 'offline' : 'online'
                      }`}
                      key={receiver.ip}
                    >
                      <label className="update-receiver-check" title={`Select ${receiver.ip}`}>
                        <input
                          type="checkbox"
                          checked={selected}
                          disabled={offline || hasRunningMultipleBatch}
                          onChange={() => {
                            if (!selected && multipleSourceMode === 'local' && multipleSharedLocalModel) {
                              setMultipleReceiverModels((models) => ({
                                ...models,
                                [receiver.ip]: multipleSharedLocalModel,
                              }))
                            }
                            setMultipleSelectedIps((current) => {
                              const next = new Set(current)
                              if (next.has(receiver.ip)) {
                                next.delete(receiver.ip)
                              } else {
                                next.add(receiver.ip)
                              }
                              return next
                            })
                          }}
                          aria-label={`Select ${receiver.ip}`}
                        />
                      </label>
                      <UpdateReceiverSummary
                        name={receiver.receiverName || receiver.ip}
                        ip={receiver.ip}
                        groupLabel={`Group ${receiver.group}`}
                        model={multipleReceiverModels[receiver.ip] || ''}
                        status={receiver.status}
                        firmwareVersion={receiver.firmwareVersion}
                        uptimeSeconds={receiver.uptimeSeconds}
                        fixType={receiver.fixType}
                        celsius={receiver.celsius}
                        svSolved={receiver.svSolved}
                      />
                      <label className="update-receiver-model-field multiple-model-field">
                        <span>Receiver Model</span>
                        <select
                          value={multipleReceiverModels[receiver.ip] || ''}
                          disabled={hasRunningMultipleBatch || multipleSourceMode === 'local'}
                          onChange={(event) =>
                            setMultipleReceiverModels((current) => ({
                              ...current,
                              [receiver.ip]: event.target.value,
                            }))
                          }
                        >
                          <option value="">Select model</option>
                          {RECEIVER_MODEL_OPTIONS.map((model) => (
                            <option value={model} key={model}>
                              {model}
                            </option>
                          ))}
                        </select>
                      </label>
                    </article>
                  )
                })}
              </div>
            </div>

            <aside className="multiple-source-panel firmware-settings-pane firmware-config-panel">
              <div className="single-card-heading">
                <Server size={18} />
                <div>
                  <h4>Firmware Source</h4>
                  <p>One managed update session applies the selected firmware across the receiver selection.</p>
                </div>
              </div>

              <div className="source-mode-switch" aria-label="Multiple firmware source mode">
                <button
                  type="button"
                  className={multipleSourceMode === 'release' ? 'active' : ''}
                  disabled={hasRunningMultipleBatch}
                  onClick={() => {
                    setMultipleSourceMode('release')
                    setMultipleFileError('')
                  }}
                >
                  <Server size={15} />
                  Release Server
                </button>
                <button
                  type="button"
                  className={multipleSourceMode === 'local' ? 'active' : ''}
                  disabled={hasRunningMultipleBatch}
                  onClick={() => {
                    setMultipleSourceMode('local')
                    setMultipleFileError('')
                  }}
                >
                  <UploadCloud size={15} />
                  Local Package
                </button>
              </div>

              {multipleSourceMode === 'release' ? (
                renderStandaloneReleaseCatalog(Boolean(hasRunningMultipleBatch))
              ) : (
                <>
                  <label className="firmware-field">
                    <span>Receiver Model</span>
                    <select
                      value={multipleSharedLocalModel}
                      disabled={hasRunningMultipleBatch || multipleSelectedReceivers.length === 0}
                      onChange={(event) => {
                        const model = event.target.value
                        setMultipleReceiverModels((current) => {
                          const next = { ...current }
                          multipleSelectedReceivers.forEach((receiver) => {
                            next[receiver.ip] = model
                          })
                          return next
                        })
                      }}
                    >
                      <option value="">Select one model for all receivers</option>
                      {RECEIVER_MODEL_OPTIONS.map((model) => (
                        <option value={model} key={model}>
                          {model}
                        </option>
                      ))}
                    </select>
                  </label>
                  <div
                    className={`firmware-dropzone multiple ${isMultipleFileDragging ? 'dragging' : ''} ${
                      multipleFirmwareFile ? 'loaded' : ''
                    } ${isMultipleFileLoading ? 'loading' : ''}`}
                    onDragOver={(event) => {
                      event.preventDefault()
                      setIsMultipleFileDragging(true)
                    }}
                    onDragLeave={() => setIsMultipleFileDragging(false)}
                    onDrop={handleMultipleFirmwareFileDrop}
                    onClick={() => void pickMultipleFirmwareFile()}
                    role="button"
                    tabIndex={0}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter' || event.key === ' ') {
                        event.preventDefault()
                        void pickMultipleFirmwareFile()
                      }
                    }}
                  >
                    <input ref={multipleFirmwareFileInputRef} type="file" accept=".img,.timg" hidden />
                    <div className="dropzone-icon">
                      {isMultipleFileLoading ? (
                        <Loader2 className="spin" size={24} />
                      ) : multipleFirmwareFile ? (
                        <FileText size={24} />
                      ) : (
                        <UploadCloud size={24} />
                      )}
                    </div>
                    <div>
                      <strong>
                        {isMultipleFileLoading
                          ? 'Loading firmware package...'
                          : multipleFirmwareFile?.name || 'Drop firmware package here'}
                      </strong>
                      <span>
                        {multipleFirmwareFile
                          ? `${formatBytes(multipleFirmwareFile.size)} - ready for multiple update`
                          : 'Click to browse or drop .img / .timg package'}
                      </span>
                    </div>
                  </div>
                  {multipleSelectedModels.size > 1 && (
                    <p className="credentials-error">
                      Select one shared hardware model before choosing the local package.
                    </p>
                  )}
                </>
              )}

              {multipleFileError && <p className="credentials-error">{multipleFileError}</p>}
              {multipleHasMissingModels && multipleSelectedIps.size > 0 && (
                <p className="credentials-error">Select a valid model for every selected receiver.</p>
              )}

              <button
                className="primary-button full-width"
                type="button"
                onClick={() => void runMultipleUpdate()}
                disabled={
                  multipleSourceMode === 'release' ? !canStartMultipleReleaseUpdate : !canStartMultipleLocalUpdate
                }
              >
                {isMultipleUpdateStarting || hasRunningMultipleBatch ? (
                  <Loader2 className="spin" size={16} />
                ) : (
                  <UploadCloud size={16} />
                )}
                Start Multiple Update
              </button>

              {firmwareError && <p className="credentials-error">{firmwareError}</p>}
            </aside>
          </section>
        </div>
      ) : page === 'FirmwareSingle' ? (
        <div className="firmware-page-scroll single-update-body firmware-operation-page">
          <section className="single-update-panel">
            <div className="single-update-grid firmware-operation-layout firmware-unified-workspace">
              <section className="single-update-card firmware-receiver-pane firmware-receiver-list-panel">
                <div className="single-card-heading">
                  <Radio size={18} />
                  <div>
                    <h4>Receiver</h4>
                    <p>Select the test receiver or type an IP manually.</p>
                  </div>
                </div>

                <label className="firmware-field">
                  <span>Receiver IP</span>
                  <input
                    value={singleReceiverIp}
                    onChange={(event) => {
                      setSingleReceiverIp(event.target.value)
                      setSingleReceiverModel('')
                    }}
                    placeholder="Search or type receiver IP"
                  />
                </label>

                {selectedSingleReceiver ? (
                  <FirmwareDeviceCard
                    className="single-selected-firmware-device-card"
                    name={selectedSingleReceiver.receiverName || selectedSingleReceiver.ip}
                    ip={selectedSingleReceiver.ip}
                    groupLabel={`Group ${selectedSingleReceiver.group}`}
                    model={effectiveSingleReceiverModel}
                    status={selectedSingleReceiver.status}
                    firmwareVersion={selectedSingleReceiver.firmwareVersion}
                    uptimeSeconds={selectedSingleReceiver.uptimeSeconds}
                    fixType={selectedSingleReceiver.fixType}
                    celsius={selectedSingleReceiver.celsius}
                    svSolved={selectedSingleReceiver.svSolved}
                    actions={
                      <button
                        className="icon-button firmware-device-card__clear"
                        type="button"
                        title="Clear receiver selection"
                        aria-label="Clear receiver selection"
                        onClick={() => {
                          setSingleReceiverIp('')
                          setSingleReceiverModel('')
                        }}
                      >
                        <X size={16} />
                      </button>
                    }
                  />
                ) : (
                  <div className="firmware-device-list single-firmware-device-list">
                    {singleReceiverOptions.map((receiver) => {
                      const configuredModel = multipleReceiverModels[receiver.ip] || ''
                      return (
                        <FirmwareDeviceCard
                          key={receiver.ip}
                          className="single-firmware-device-card"
                          name={receiver.receiverName || receiver.ip}
                          ip={receiver.ip}
                          groupLabel={`Group ${receiver.group}`}
                          model={configuredModel}
                          status={receiver.status}
                          firmwareVersion={receiver.firmwareVersion}
                          uptimeSeconds={receiver.uptimeSeconds}
                          fixType={receiver.fixType}
                          celsius={receiver.celsius}
                          svSolved={receiver.svSolved}
                          onSelect={() => {
                            setSingleReceiverIp(receiver.ip)
                            setSingleReceiverModel(configuredModel)
                          }}
                          ariaLabel={`Select receiver ${receiver.ip}`}
                          actions={<ChevronRight className="firmware-device-card__chevron" size={18} aria-hidden="true" />}
                        />
                      )
                    })}
                  </div>
                )}
              </section>

              <section className="single-update-card firmware-settings-pane firmware-config-panel">
                <div className="single-card-heading">
                  <Server size={18} />
                  <div>
                    <h4>Firmware Source</h4>
                    <p>Use the release server or stage a local package for the next backend phase.</p>
                  </div>
                </div>

                <div className="source-mode-switch" aria-label="Firmware source mode">
                  <button
                    type="button"
                    className={singleSourceMode === 'release' ? 'active' : ''}
                    onClick={() => {
                      setSingleSourceMode('release')
                      setSingleFileError('')
                    }}
                  >
                    <Server size={15} />
                    Release Server
                  </button>
                  <button
                    type="button"
                    className={singleSourceMode === 'local' ? 'active' : ''}
                    onClick={() => {
                      setSingleSourceMode('local')
                      setSingleFileError('')
                    }}
                  >
                    <UploadCloud size={15} />
                    Local Package
                  </button>
                </div>

                {singleSourceMode === 'release' ? (
                  <div className="standalone-source-fields">
                    <label className="firmware-field">
                      <span>Receiver Model</span>
                      <select
                        value={effectiveSingleReceiverModel}
                        disabled={isSingleUpdateStarting}
                        onChange={(event) => setSingleReceiverModel(event.target.value)}
                      >
                        <option value="">Select receiver model</option>
                        {RECEIVER_MODEL_OPTIONS.map((model) => (
                          <option value={model} key={model}>
                            {model.toUpperCase()}
                          </option>
                        ))}
                      </select>
                    </label>
                    {renderStandaloneReleaseCatalog(isSingleUpdateStarting)}
                  </div>
                ) : (
                  <div
                    className={`firmware-dropzone ${isFirmwareFileDragging ? 'dragging' : ''} ${
                      singleFirmwareFile ? 'loaded' : ''
                    } ${isSingleFileLoading ? 'loading' : ''}`}
                    onDragOver={(event) => {
                      event.preventDefault()
                      setIsFirmwareFileDragging(true)
                    }}
                    onDragLeave={() => setIsFirmwareFileDragging(false)}
                    onDrop={handleFirmwareFileDrop}
                    onClick={() => void pickSingleFirmwareFile()}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter' || event.key === ' ') {
                        event.preventDefault()
                        void pickSingleFirmwareFile()
                      }
                    }}
                    role="button"
                    tabIndex={0}
                  >
                    <input
                      ref={firmwareFileInputRef}
                      type="file"
                      accept=".img,.timg"
                      hidden
                      onChange={(event) => selectSingleFirmwareFile(event.target.files?.item(0) ?? null)}
                    />
                    <div className="dropzone-icon">
                      {isSingleFileLoading ? (
                        <Loader2 className="spin" size={24} />
                      ) : singleFirmwareFile ? (
                        <FileText size={24} />
                      ) : (
                        <UploadCloud size={24} />
                      )}
                    </div>
                    <div>
                      <strong>
                        {isSingleFileLoading
                          ? 'Loading firmware package...'
                          : singleFirmwareFile
                            ? singleFirmwareFile.name
                            : 'Drop firmware package here'}
                      </strong>
                      <span>
                        {isSingleFileLoading
                          ? 'Validating file path and package format'
                          : singleFirmwareFile
                          ? `${formatBytes(singleFirmwareFile.size)} - ${singleFirmwareFilePath ? 'ready for local update' : 'path unavailable'}`
                          : 'Click to browse or drop .img / .timg package'}
                      </span>
                    </div>
                  </div>
                )}

                {singleFileError && <p className="credentials-error">{singleFileError}</p>}

                <div className="single-source-actions">
                  <div className="single-source-capacity">
                    <span>{activeSingleUpdateJobs.length} / 3 updates running</span>
                    {selectedReceiverHasRunningUpdate && <strong>Receiver update already in progress</strong>}
                  </div>

                  <div className="single-source-action-grid">
                    <button
                      className="primary-button"
                      type="button"
                      onClick={() =>
                        void (singleSourceMode === 'release' ? runSingleReleaseUpdate() : runSingleLocalUpdate())
                      }
                      disabled={
                        singleSourceMode === 'release' ? !canStartSingleReleaseUpdate : !canStartSingleLocalUpdate
                      }
                    >
                      {isSingleUpdateStarting ? (
                        <Loader2 className="spin" size={16} />
                      ) : singleSourceMode === 'release' ? (
                        <Server size={16} />
                      ) : (
                        <UploadCloud size={16} />
                      )}
                      {singleSourceMode === 'release' ? 'Start Release Update' : 'Start Local Update'}
                    </button>

                    <button
                      className="secondary-button"
                      type="button"
                      onClick={() => void restoreSelectedReceiverPassword()}
                      disabled={!selectedSingleReceiver || isPasswordRestoreRunning}
                    >
                      {isPasswordRestoreRunning ? <Loader2 className="spin" size={16} /> : <ShieldCheck size={16} />}
                      Restore Password
                    </button>
                  </div>

                  {passwordRestoreResult && (
                    <p className="single-recovery-result">
                      {passwordRestoreResult.status}: {passwordRestoreResult.message}
                    </p>
                  )}
                </div>

                {firmwareError && <p className="credentials-error">{firmwareError}</p>}
              </section>
            </div>
          </section>
        </div>
      ) : (
        <div className="firmware-page-content create-group-page">
          <section className="firmware-plan-toolbar create-group-toolbar">
            <div className="firmware-panel-header">
              <div>
                <h3>Group Details</h3>
                <p>Name the group and choose the receivers that should belong to it.</p>
              </div>
              <span className="multiple-selection-count">{newGroupSelectedIps.size} selected</span>
            </div>
            <div className="create-group-fields">
              <label className="firmware-field">
                <span>Group Name</span>
                <input
                  value={newGroupName}
                  onChange={(event) => setNewGroupName(event.target.value)}
                  placeholder="Example: Integration Lab"
                />
              </label>
              <label className="history-search create-group-search">
                <Search size={17} />
                <input
                  value={newGroupQuery}
                  onChange={(event) => setNewGroupQuery(event.target.value)}
                  placeholder="Search IP, receiver, firmware or fix type"
                />
              </label>
            </div>
          </section>

          <section className="firmware-list-panel">
            <div className="firmware-selection-toolbar">
              <div className="firmware-list-heading">
                <h3>Available Receivers</h3>
                <span>{newGroupReceiverOptions.length} visible</span>
              </div>
              <div>
                <button
                  className="secondary-button"
                  type="button"
                  disabled={!newGroupReceiverOptions.length}
                  onClick={() =>
                    setNewGroupSelectedIps((current) => {
                      const visibleIps = newGroupReceiverOptions.map((receiver) => receiver.ip)
                      const allSelected = visibleIps.every((ip) => current.has(ip))
                      const next = new Set(current)
                      visibleIps.forEach((ip) => (allSelected ? next.delete(ip) : next.add(ip)))
                      return next
                    })
                  }
                >
                  {newGroupReceiverOptions.length > 0 &&
                  newGroupReceiverOptions.every((receiver) => newGroupSelectedIps.has(receiver.ip))
                    ? 'Clear Visible'
                    : 'Select Visible'}
                </button>
                <button
                  className="secondary-button"
                  type="button"
                  disabled={!newGroupSelectedIps.size}
                  onClick={() => setNewGroupSelectedIps(new Set())}
                >
                  Clear
                </button>
              </div>
            </div>
            <div className="create-group-receiver-list">
              {newGroupReceiverOptions.map((receiver) => (
                <label
                  className={`create-group-receiver-row ${newGroupSelectedIps.has(receiver.ip) ? 'selected' : ''}`}
                  key={receiver.ip}
                >
                  <input
                    type="checkbox"
                    checked={newGroupSelectedIps.has(receiver.ip)}
                    onChange={() =>
                      setNewGroupSelectedIps((current) => {
                        const next = new Set(current)
                        if (next.has(receiver.ip)) next.delete(receiver.ip)
                        else next.add(receiver.ip)
                        return next
                      })
                    }
                  />
                  <span className={`status-dot ${receiver.status}`} />
                  <div>
                    <strong>{receiver.receiverName || receiver.ip}</strong>
                    <span>{receiver.ip} - Group {receiver.group}</span>
                  </div>
                  <span className="firmware-mini-stat">
                    <Gauge size={14} />
                    FW {receiver.firmwareVersion || '--'}
                  </span>
                </label>
              ))}
              {!newGroupReceiverOptions.length && (
                <div className="firmware-empty">
                  <Radio size={34} />
                  <strong>No receivers found</strong>
                  <p>Change the search or refresh the receiver list.</p>
                </div>
              )}
            </div>
          </section>

          <footer className="firmware-action-dock create-group-actions">
            <div className="firmware-operation-copy">
              <strong>{newGroupName.trim() || 'New receiver group'}</strong>
              <span>{newGroupSelectedIps.size} receivers ready for the future group configuration backend.</span>
            </div>
            <button className="primary-button" type="button" disabled>
              <Plus size={16} />
              Save Group
            </button>
          </footer>
        </div>
      )}

      {isGroupStartModalOpen && isGroupUpdate &&
        createPortal(
          <div className="modal-layer">
            <button
              className="modal-backdrop"
              type="button"
              aria-label="Close group update confirmation"
              onClick={() => setIsGroupStartModalOpen(false)}
            />
            <section className="firmware-config-modal group-update-confirm-modal" aria-label="Confirm group update">
              <header className="credentials-header">
                <div className="credentials-title">
                  <div className="credentials-icon">
                    <UploadCloud size={20} />
                  </div>
                  <div>
                    <h2>Start Group Update</h2>
                    <p>Review the update scope before receiver changes begin.</p>
                  </div>
                </div>
                <button
                  className="modal-close"
                  type="button"
                  aria-label="Close group update confirmation"
                  onClick={() => setIsGroupStartModalOpen(false)}
                >
                  <X size={18} />
                </button>
              </header>

              <div className="firmware-config-modal-body">
                <div className="group-update-review" role="list" aria-label="Group update summary">
                  <div className="group-update-review-row" role="listitem">
                    <span>Receiver group</span>
                    <strong>{current.title}</strong>
                  </div>
                  <div className="group-update-review-row" role="listitem">
                    <span>Selected receivers</span>
                    <strong>{selectedIps.size}</strong>
                  </div>
                  <div className="group-update-review-row" role="listitem">
                    <span>Target release</span>
                    <strong>
                      {groupReleaseMode === 'catalog'
                        ? `${activeSelectedReleaseId} / ${activeSelectedBuildId}`
                        : `${selectedGroupModels.length} local model packages`}
                    </strong>
                  </div>
                  <div className="group-update-review-row" role="listitem">
                    <span>Processing mode</span>
                    <strong>{parallel ? `Up to ${Math.min(3, selectedIps.size)} receivers` : 'One receiver at a time'}</strong>
                  </div>
                </div>

                <div className="group-update-safety-note">
                  <ShieldCheck size={20} />
                  <p>
                    Each receiver gets a configuration backup before upload. Its password is changed temporarily and
                    restored after the receiver returns, including failure recovery paths.
                  </p>
                </div>

                <div className="credentials-actions">
                  <button className="secondary-button" type="button" onClick={() => setIsGroupStartModalOpen(false)}>
                    Cancel
                  </button>
                  <button
                    className="primary-button"
                    type="button"
                    onClick={() => void prepareJob()}
                    disabled={!canRunPlan || isJobStarting}
                  >
                    {isJobStarting ? <Loader2 className="spin" size={16} /> : <UploadCloud size={16} />}
                    Start Update
                  </button>
                </div>
              </div>
            </section>
          </div>,
          document.body,
        )}

      {isAddReceiverModalOpen &&
        createPortal(
          <div className="modal-layer">
            <button
              className="modal-backdrop"
              type="button"
              aria-label="Close receiver model dialog"
              onClick={() => {
                setIsAddReceiverModalOpen(false)
                setReceiverPendingModelEdit(null)
              }}
            />
            <section className="firmware-config-modal" aria-label="Receiver firmware model">
              <header className="credentials-header">
                <div className="credentials-title">
                  <div className="credentials-icon">
                    {receiverPendingModelEdit ? <Pencil size={20} /> : <Plus size={20} />}
                  </div>
                  <div>
                    <h2>{receiverPendingModelEdit ? 'Edit Receiver Model' : 'Add Config Receiver'}</h2>
                    <p>
                      {receiverPendingModelEdit
                        ? `Assign the correct firmware engine for ${receiverPendingModelEdit.ip}.`
                        : 'Select a loaded receiver or type an IP manually.'}
                    </p>
                  </div>
                </div>
                <button
                  className="modal-close"
                  type="button"
                  aria-label="Close receiver model dialog"
                  onClick={() => {
                    setIsAddReceiverModalOpen(false)
                    setReceiverPendingModelEdit(null)
                  }}
                >
                  <X size={18} />
                </button>
              </header>

              <div className="firmware-config-modal-body">
                <div className="firmware-config-note">
                  <h3>Update Scope</h3>
                  <p>
                    Group membership comes from qa-receivers.json. This action assigns the firmware model used by{' '}
                    {current.title} updates.
                  </p>
                </div>

                <div className="firmware-modal-form">
                  <label className="firmware-field">
                    <span>Receiver IP</span>
                    <input
                      autoFocus={!receiverPendingModelEdit}
                      value={newReceiverIp}
                      onChange={(event) => setNewReceiverIp(event.target.value)}
                      placeholder="Search or type IP"
                      disabled={Boolean(receiverPendingModelEdit)}
                    />
                  </label>

                  {!receiverPendingModelEdit ? <div className="receiver-picker-list" aria-label="Loaded receivers">
                    {receiverPickerOptions.map((receiver) => (
                      <button
                        type="button"
                        key={receiver.ip}
                        onClick={() => setNewReceiverIp(receiver.ip)}
                        className={`${newReceiverIp.trim() === receiver.ip ? 'active' : ''} ${
                          configuredIpSet.has(receiver.ip) ? 'configured' : ''
                        }`}
                      >
                        <strong>{receiver.ip}</strong>
                        <span>
                          Group {receiver.group}
                          {receiver.receiverName ? ` - ${receiver.receiverName}` : ''}
                        </span>
                        <small>{configuredIpSet.has(receiver.ip) ? 'Configured' : receiver.firmwareVersion || 'FW --'}</small>
                      </button>
                    ))}
                    {!receiverPickerOptions.length && (
                      <p className="receiver-picker-empty">No matching loaded receivers. You can still type IP manually.</p>
                    )}
                  </div> : null}

                  <label className="firmware-field">
                    <span>Receiver Model</span>
                    <select
                      autoFocus={Boolean(receiverPendingModelEdit)}
                      value={newReceiverModel}
                      onChange={(event) => setNewReceiverModel(event.target.value)}
                    >
                      <option value="">Select receiver model</option>
                      {RECEIVER_MODEL_OPTIONS.map((model) => (
                        <option value={model} key={model}>
                          {model.toUpperCase()}
                        </option>
                      ))}
                    </select>
                  </label>

                  {firmwareError && <p className="credentials-error">{firmwareError}</p>}

                  <div className="credentials-actions">
                    <button
                      className="secondary-button"
                      type="button"
                      onClick={() => {
                        setIsAddReceiverModalOpen(false)
                        setReceiverPendingModelEdit(null)
                      }}
                    >
                      Cancel
                    </button>
                    <button
                      className="primary-button"
                      type="button"
                      onClick={() => void addReceiver()}
                      disabled={isSavingReceiver || !newReceiverIp.trim() || !newReceiverModel.trim()}
                    >
                      {isSavingReceiver ? (
                        <Loader2 className="spin" size={16} />
                      ) : receiverPendingModelEdit ? (
                        <Pencil size={16} />
                      ) : (
                        <Plus size={16} />
                      )}
                      {receiverPendingModelEdit ? 'Save Model' : 'Add Receiver'}
                    </button>
                  </div>
                </div>
              </div>
            </section>
          </div>,
          document.body,
        )}

      {singleUpdateJobs.length > 0 &&
        createPortal(
          <>
            {singleUpdateOrbJobs.length > 0 && (
              <div className="single-update-orb-stack">
                {singleUpdateOrbJobs.map((job) => {
                  const progress = getSingleUpdateProgress(job)
                  return (
                    <button
                      className={`single-update-orb ${job.status.toLowerCase()}`}
                      type="button"
                      key={job.jobId}
                      style={{ '--progress': `${progress}%` } as CSSProperties}
                      onClick={() => {
                        setActiveSingleUpdateJobId(job.jobId)
                        setMinimizedSingleUpdateJobIds((current) => {
                          const next = new Set(current)
                          next.delete(job.jobId)
                          return next
                        })
                      }}
                      aria-label={`Restore firmware update status for ${job.ip}`}
                    >
                      <strong>{formatProgressPercent(progress)}</strong>
                    </button>
                  )
                })}
              </div>
            )}

            {activeSingleUpdateJob && !minimizedSingleUpdateJobIds.has(activeSingleUpdateJob.jobId) && (
              <div className="modal-layer">
                <button
                  className="modal-backdrop"
                  type="button"
                  aria-label="Minimize firmware update status"
                  onClick={() => {
                    if (!canDismissSingleUpdateStatus(activeSingleUpdateJob)) {
                      setMinimizedSingleUpdateJobIds((current) => new Set(current).add(activeSingleUpdateJob.jobId))
                    }
                  }}
                />
                <section className="single-update-modal" aria-label="Single firmware update status">
                  <header className="credentials-header">
                    <div className="credentials-title">
                      <div className="credentials-icon">
                        {activeSingleUpdateJob.status === 'Running' ? (
                          <Loader2 className="spin" size={20} />
                        ) : activeSingleUpdateJob.status === 'Completed' ? (
                          <ShieldCheck size={20} />
                        ) : (
                          <AlertTriangle size={20} />
                        )}
                      </div>
                      <div>
                        <h2>Single Firmware Update</h2>
                        <p>
                          {activeSingleUpdateJob.receiverName || activeSingleUpdateJob.ip} - {activeSingleUpdateJob.targetFile}
                        </p>
                      </div>
                    </div>
                    <button
                      className="modal-close"
                      type="button"
                      aria-label={
                        canDismissSingleUpdateStatus(activeSingleUpdateJob)
                          ? 'Close firmware update status'
                          : 'Minimize firmware update status'
                      }
                      onClick={() => {
                        if (canDismissSingleUpdateStatus(activeSingleUpdateJob)) {
                          dismissSingleUpdateStatus(activeSingleUpdateJob.jobId)
                        } else {
                          setMinimizedSingleUpdateJobIds((current) =>
                            new Set(current).add(activeSingleUpdateJob.jobId),
                          )
                        }
                      }}
                    >
                      {canDismissSingleUpdateStatus(activeSingleUpdateJob) ? <X size={18} /> : <Minimize2 size={18} />}
                    </button>
                  </header>

                  <div className="single-update-modal-body">
                    {activeSingleUpdateJob.downgradeRequired &&
                      activeSingleUpdateJob.downgradeDecision === null && (
                        <div className="firmware-downgrade-gate" role="alert">
                          <div className="firmware-downgrade-copy">
                            <AlertTriangle size={20} />
                            <div>
                              <strong>Downgrade approval required</strong>
                              <p>
                                Open the receiver Firmware page and choose OK for the downgrade warning. Then continue
                                here, or cancel this receiver.
                              </p>
                            </div>
                          </div>
                          <div className="firmware-downgrade-actions">
                            <button
                              className="secondary-button"
                              type="button"
                              onClick={() => void openDowngradeReceiver(activeSingleUpdateJob.ip)}
                            >
                              <ExternalLink size={16} />
                              Open Firmware Page
                            </button>
                            <button
                              className="secondary-button"
                              type="button"
                              disabled={resolvingDowngradeJobIds.has(activeSingleUpdateJob.jobId)}
                              onClick={() => void resolveDowngrade(activeSingleUpdateJob.jobId, true, false)}
                            >
                              <ShieldCheck size={16} />
                              Continue Update
                            </button>
                            <button
                              className="danger-button"
                              type="button"
                              disabled={resolvingDowngradeJobIds.has(activeSingleUpdateJob.jobId)}
                              onClick={() => void resolveDowngrade(activeSingleUpdateJob.jobId, false, false)}
                            >
                              <X size={16} />
                              Cancel Receiver
                            </button>
                          </div>
                        </div>
                      )}
                    <div className={`single-update-hero ${activeSingleUpdateJob.status.toLowerCase()}`}>
                      <span className={`preflight-pill ${activeSingleUpdateJob.status.toLowerCase()}`}>
                        {activeSingleUpdateJob.status}
                      </span>
                      <h3>
                        {getFirmwareJobDisplayState(activeSingleUpdateJob)}
                      </h3>
                      <p>
                        {activeSingleUpdateJob.status === 'Completed'
                          ? 'Receiver is online and the configured password has been restored.'
                          : isFirmwareWorkflowMilestone(activeSingleUpdateJob.phase)
                            ? activeSingleUpdateJob.message
                            : activeSingleUpdateJob.updateMode || activeSingleUpdateJob.fwStatus
                              ? 'Live firmware progress is being read from the receiver.'
                              : activeSingleUpdateJob.message}
                      </p>
                      <div className="single-update-progress">
                        <div className="single-update-progress-label">
                          <span>
                            {activeSingleUpdateJob.status === 'Completed'
                              ? '100% complete'
                              : activeSingleUpdateJob.updatePercent !== null
                                ? `${formatProgressPercent(activeSingleUpdateJob.updatePercent)} complete`
                                : 'Waiting for receiver progress'}
                          </span>
                          <strong>
                            {activeSingleUpdateJob.status === 'Completed'
                              ? 'Finished'
                              : formatWorkflowState(activeSingleUpdateJob.phase)}
                          </strong>
                        </div>
                        <div className="single-update-progress-track">
                          <div
                            style={{
                              width: `${getSingleUpdateProgress(activeSingleUpdateJob)}%`,
                            }}
                          />
                        </div>
                      </div>
                    </div>

                    <div className="single-update-live-grid">
                      <div>
                        <span>Receiver</span>
                        <strong>{activeSingleUpdateJob.ip}</strong>
                      </div>
                      <div>
                        <span>Current FW</span>
                        <strong>{activeSingleUpdateJob.firmwareVersion || '--'}</strong>
                      </div>
                      <div>
                        <span>Target Package</span>
                        <strong>{activeSingleUpdateJob.targetFile}</strong>
                      </div>
                    </div>
                  </div>
                </section>
              </div>
            )}
          </>,
          document.body,
        )}

      {multipleBatch &&
        createPortal(
          <>
            {isMultipleBatchMinimized && (
              <button
                className={`single-update-orb multiple-update-orb ${multipleBatch.status.toLowerCase()}`}
                type="button"
                style={
                  {
                    '--progress': `${multipleOverallProgress}%`,
                    bottom: `${24 + singleUpdateOrbJobs.length * 76}px`,
                  } as CSSProperties
                }
                onClick={() => setIsMultipleBatchMinimized(false)}
                aria-label="Restore multiple receiver update status"
              >
                <strong>{formatProgressPercent(multipleOverallProgress)}</strong>
              </button>
            )}

            {!isMultipleBatchMinimized && (
              <div className="modal-layer">
                <button
                  className="modal-backdrop"
                  type="button"
                  aria-label="Minimize multiple receiver update status"
                  onClick={() => {
                    if (!canDismissMultipleBatch) {
                      setIsMultipleBatchMinimized(true)
                    }
                  }}
                />
                <section className="multiple-update-modal" aria-label="Multiple receiver update status">
                  <header className="credentials-header">
                    <div className="credentials-title">
                      <div className="credentials-icon">
                        {multipleBatch.status === 'Running' ? (
                          <Loader2 className="spin" size={20} />
                        ) : multipleBatch.status === 'Completed' ? (
                          <ShieldCheck size={20} />
                        ) : (
                          <AlertTriangle size={20} />
                        )}
                      </div>
                      <div>
                        <h2>Multiple Receivers Update</h2>
                        <p>
                          {multipleBatch.items.length} receivers - {multipleBatch.targetLabel}
                        </p>
                      </div>
                    </div>
                    <button
                      className="modal-close"
                      type="button"
                      aria-label={canDismissMultipleBatch ? 'Close multiple update status' : 'Minimize multiple update status'}
                      onClick={() => {
                        if (canDismissMultipleBatch) {
                          setMultipleBatch(null)
                          setIsMultipleBatchMinimized(false)
                        } else {
                          setIsMultipleBatchMinimized(true)
                        }
                      }}
                    >
                      {canDismissMultipleBatch ? <X size={18} /> : <Minimize2 size={18} />}
                    </button>
                  </header>

                  <div className="multiple-update-modal-body">
                    <div className={`multiple-update-overview ${multipleBatch.status.toLowerCase()}`}>
                      <div className="multiple-update-overview-heading">
                        <div>
                          <span className={`preflight-pill ${multipleBatch.status.toLowerCase()}`}>
                            {multipleBatch.status}
                          </span>
                          <h3>
                            {multipleBatch.status === 'Running'
                              ? 'Receiver update queue is active'
                              : multipleBatch.status === 'Completed'
                                ? 'All receiver updates completed'
                                : 'Multiple update finished with errors'}
                          </h3>
                        </div>
                        <strong>{formatProgressPercent(multipleOverallProgress)}</strong>
                      </div>
                      <div className="single-update-progress-track">
                        <div style={{ width: `${multipleOverallProgress}%` }} />
                      </div>
                      <div className="multiple-update-counters">
                        <span>{multipleCompletedCount} completed</span>
                        <span>{multipleFailedCount} failed</span>
                        <span>
                          {multipleBatch.items.length - multipleCompletedCount - multipleFailedCount} active / queued
                        </span>
                      </div>
                    </div>

                    <div className="multiple-update-status-list">
                      {multipleBatch.items.map((item) => {
                        const expanded = expandedMultipleIps.has(item.ip)
                        return (
                          <article
                            className={`multiple-update-status-item ${item.status.toLowerCase()}`}
                            key={item.ip}
                          >
                            <button
                              type="button"
                              className="multiple-update-status-main"
                              onClick={() =>
                                setExpandedMultipleIps((current) => {
                                  const next = new Set(current)
                                  if (next.has(item.ip)) {
                                    next.delete(item.ip)
                                  } else {
                                    next.add(item.ip)
                                  }
                                  return next
                                })
                              }
                            >
                              <ChevronRight className={expanded ? 'expanded' : ''} size={17} />
                              <div>
                                <strong>{item.receiverName || item.ip}</strong>
                                <span>
                                  {item.ip} - {item.model}
                                </span>
                              </div>
                              <div className="multiple-item-progress">
                                <span>{formatProgressPercent(item.progress)}</span>
                                <div>
                                  <i style={{ width: `${item.progress}%` }} />
                                </div>
                              </div>
                              <span className={`preflight-pill ${item.status.toLowerCase()}`}>{item.status}</span>
                            </button>
                            {expanded && (
                              <div className="multiple-update-status-details">
                                <div>
                                  <span>Current FW</span>
                                  <strong>{item.currentFirmware || '--'}</strong>
                                </div>
                                <div>
                                  <span>Target Package</span>
                                  <strong>{item.targetFile}</strong>
                                </div>
                                <div>
                                  <span>Current Phase</span>
                                  <strong>{formatWorkflowState(item.phase)}</strong>
                                </div>
                                <p>{item.message}</p>
                              </div>
                            )}
                          </article>
                        )
                      })}
                    </div>
                  </div>
                </section>
              </div>
            )}
          </>,
          document.body,
        )}

      {jobStatus &&
        createPortal(
          <>
            {isGroupJobMinimized && (
              <button
                className={`single-update-orb multiple-update-orb group-update-orb ${jobStatus.status.toLowerCase()}`}
                type="button"
                style={
                  {
                    '--progress': `${jobStatus.progress}%`,
                    bottom: `${24 + singleUpdateOrbJobs.length * 76 + (isMultipleBatchMinimized ? 106 : 0)}px`,
                  } as CSSProperties
                }
                onClick={() => setIsGroupJobMinimized(false)}
                aria-label="Restore group firmware update status"
              >
                <strong>{formatProgressPercent(jobStatus.progress)}</strong>
              </button>
            )}

            {!isGroupJobMinimized && (
              <div className="modal-layer">
                <button
                  className="modal-backdrop"
                  type="button"
                  aria-label="Minimize group firmware update status"
                  onClick={() => {
                    if (!canDismissGroupJob) {
                      setIsGroupJobMinimized(true)
                    }
                  }}
                />
                <section className="multiple-update-modal" aria-label="Group firmware update status">
                  <header className="credentials-header">
                    <div className="credentials-title">
                      <div className="credentials-icon">
                        {jobStatus.status === 'Completed' ? (
                          <ShieldCheck size={20} />
                        ) : jobStatus.status === 'Failed' ? (
                          <AlertTriangle size={20} />
                        ) : (
                          <Loader2 className="spin" size={20} />
                        )}
                      </div>
                      <div>
                        <h2>{jobStatus.groupName} Firmware Update</h2>
                        <p>
                          {jobStatus.receivers.length} receivers - {jobStatus.firmwareVersion}
                        </p>
                      </div>
                    </div>
                    <button
                      className="modal-close"
                      type="button"
                      aria-label={canDismissGroupJob ? 'Close group update status' : 'Minimize group update status'}
                      onClick={() => {
                        if (canDismissGroupJob) {
                          setJobStatus(null)
                          setIsGroupJobMinimized(false)
                          setExpandedGroupIps(new Set())
                        } else {
                          setIsGroupJobMinimized(true)
                        }
                      }}
                    >
                      {canDismissGroupJob ? <X size={18} /> : <Minimize2 size={18} />}
                    </button>
                  </header>

                  <div className="multiple-update-modal-body">
                    <div className={`multiple-update-overview ${jobStatus.status.toLowerCase()}`}>
                      <div className="multiple-update-overview-heading">
                        <div>
                          <span className={`preflight-pill ${jobStatus.status.toLowerCase()}`}>{jobStatus.status}</span>
                          <h3>
                            {jobStatus.status === 'Completed'
                              ? 'All receiver updates completed'
                              : jobStatus.status === 'Failed'
                                ? 'Group update finished with errors'
                                : jobStatus.status === 'Cancelled'
                                  ? 'Group update was cancelled and receiver reservations were released'
                                : jobStatus.status === 'Cancelling'
                                  ? 'Stopping safely and restoring passwords'
                                  : 'Group receiver queue is active'}
                          </h3>
                        </div>
                        <strong>{formatProgressPercent(jobStatus.progress)}</strong>
                      </div>
                      <div className="single-update-progress-track">
                        <div style={{ width: `${Math.max(0, Math.min(100, jobStatus.progress))}%` }} />
                      </div>
                      <div className="multiple-update-counters">
                        <span>{groupCompletedCount} completed</span>
                        <span>{groupCancelledCount} cancelled</span>
                        <span>{groupFailedCount} failed</span>
                        <span>
                          {jobStatus.receivers.length - groupCompletedCount - groupCancelledCount - groupFailedCount}{' '}
                          active / queued
                        </span>
                      </div>
                    </div>

                    <div className="multiple-update-status-list">
                      {jobStatus.receivers.map((receiver) => {
                        const expanded = expandedGroupIps.has(receiver.ip)
                        return (
                          <article className={`multiple-update-status-item ${receiver.status.toLowerCase()}`} key={receiver.ip}>
                            <button
                              type="button"
                              className="multiple-update-status-main"
                              onClick={() =>
                                setExpandedGroupIps((current) => {
                                  const next = new Set(current)
                                  if (next.has(receiver.ip)) next.delete(receiver.ip)
                                  else next.add(receiver.ip)
                                  return next
                                })
                              }
                            >
                              <ChevronRight className={expanded ? 'expanded' : ''} size={17} />
                              <div>
                                <strong>{receiver.ip}</strong>
                                <span>{receiver.model}</span>
                              </div>
                              <div className="multiple-item-progress">
                                <span>{formatProgressPercent(receiver.progress)}</span>
                                <div>
                                  <i style={{ width: `${Math.max(0, Math.min(100, receiver.progress))}%` }} />
                                </div>
                              </div>
                              <span className={`preflight-pill ${receiver.status.toLowerCase()}`}>{receiver.status}</span>
                            </button>
                            {receiver.downgradeRequired && receiver.downgradeDecision === null && (
                              <div className="firmware-downgrade-gate compact" role="alert">
                                <div className="firmware-downgrade-copy">
                                  <AlertTriangle size={18} />
                                  <div>
                                    <strong>Downgrade approval required</strong>
                                    <p>
                                      Open this receiver, confirm the downgrade warning on its Firmware page, then
                                      continue here. Cancel affects only {receiver.ip}.
                                    </p>
                                  </div>
                                </div>
                                <div className="firmware-downgrade-actions">
                                  <button
                                    className="secondary-button"
                                    type="button"
                                    onClick={() => void openDowngradeReceiver(receiver.ip)}
                                  >
                                    <ExternalLink size={15} />
                                    Open Firmware Page
                                  </button>
                                  <button
                                    className="secondary-button"
                                    type="button"
                                    disabled={resolvingDowngradeJobIds.has(receiver.childJobId)}
                                    onClick={() => void resolveDowngrade(receiver.childJobId, true, true)}
                                  >
                                    {resolvingDowngradeJobIds.has(receiver.childJobId) ? (
                                      <Loader2 className="spin" size={15} />
                                    ) : (
                                      <ShieldCheck size={15} />
                                    )}
                                    Continue Update
                                  </button>
                                  <button
                                    className="danger-button"
                                    type="button"
                                    disabled={resolvingDowngradeJobIds.has(receiver.childJobId)}
                                    onClick={() => void resolveDowngrade(receiver.childJobId, false, true)}
                                  >
                                    <X size={15} />
                                    Cancel Receiver
                                  </button>
                                </div>
                              </div>
                            )}
                            {expanded && (
                              <div className="multiple-update-status-details">
                                <div>
                                  <span>Current FW</span>
                                  <strong>{receiver.currentFirmware || '--'}</strong>
                                </div>
                                <div>
                                  <span>Target Package</span>
                                  <strong>{receiver.targetFile}</strong>
                                </div>
                                <div>
                                  <span>Current Phase</span>
                                  <strong>
                                    {receiver.phase === 'Preflight'
                                      ? 'Receiver Check'
                                      : formatWorkflowState(receiver.phase)}
                                  </strong>
                                </div>
                                <p>{receiver.message}</p>
                              </div>
                            )}
                          </article>
                        )
                      })}
                    </div>
                  </div>
                  {!canDismissGroupJob && (
                    <footer className="multiple-update-modal-footer">
                      <div>
                        <strong>Password recovery and cancellation</strong>
                        <span>
                          {groupPasswordRecoveryResult
                            ? `${groupPasswordRecoveryResult.restoredCount} restored, ${groupPasswordRecoveryResult.alreadyRestoredCount} already configured, ${groupPasswordRecoveryResult.deferredCount} deferred, ${groupPasswordRecoveryResult.failedCount} failed.`
                            : 'Cancel stops queued work and releases active receivers after firmware becomes idle. Forced recovery skips receivers that are still programming.'}
                        </span>
                      </div>
                      <div className="group-update-footer-actions">
                        <button
                          className="secondary-button group-password-recovery-button"
                          type="button"
                          disabled={isRestoringGroupPasswords}
                          onClick={() => void forceRestoreGroupPasswords()}
                        >
                          {isRestoringGroupPasswords ? (
                            <Loader2 className="spin" size={16} />
                          ) : (
                            <KeyRound size={16} />
                          )}
                          Force Restore Passwords
                        </button>
                        <button
                          className="danger-button group-update-cancel-button"
                          type="button"
                          disabled={isStoppingGroupJob || jobStatus.status === 'Cancelling'}
                          onClick={() => void stopJob()}
                        >
                          {isStoppingGroupJob || jobStatus.status === 'Cancelling' ? (
                            <Loader2 className="spin" size={16} />
                          ) : (
                            <X size={16} />
                          )}
                          {jobStatus.status === 'Cancelling' ? 'Cancelling Safely' : 'Cancel Update'}
                        </button>
                      </div>
                    </footer>
                  )}
                </section>
              </div>
            )}
          </>,
          document.body,
        )}

      {receiverPendingRemoval &&
        createPortal(
          <div className="modal-layer">
            <button
              className="modal-backdrop"
              type="button"
              aria-label="Close remove receiver dialog"
              onClick={() => {
                setReceiverPendingRemoval(null)
                setRemovePassword('')
                setShowRemovePassword(false)
              }}
            />
            <section className="firmware-confirm-modal" aria-label="Remove config receiver">
              <header className="credentials-header danger">
                <div className="credentials-title">
                  <div className="credentials-icon danger">
                    <X size={20} />
                  </div>
                  <div>
                    <h2>Remove Config Receiver</h2>
                    <p>This action changes the firmware group configuration.</p>
                  </div>
                </div>
                <button
                  className="modal-close"
                  type="button"
                  aria-label="Close remove receiver dialog"
                  onClick={() => {
                    setReceiverPendingRemoval(null)
                    setRemovePassword('')
                    setShowRemovePassword(false)
                  }}
                >
                  <X size={18} />
                </button>
              </header>

              <div className="credentials-form">
                <div className="remove-target-card">
                  <span>Receiver</span>
                  <strong>{receiverPendingRemoval.ip}</strong>
                  <p>
                    {receiverPendingRemoval.groupName} - {receiverPendingRemoval.model}
                  </p>
                </div>

                <p className="credentials-note">
                  Re-enter your application password to remove this receiver from the firmware config.
                </p>

                <label className="credentials-field">
                  <span>Application Password</span>
                  <div>
                    <Lock size={17} />
                    <input
                      autoFocus
                      type={showRemovePassword ? 'text' : 'password'}
                      value={removePassword}
                      onChange={(event) => setRemovePassword(event.target.value)}
                      placeholder="Enter your password"
                    />
                    <button
                      type="button"
                      className="password-toggle"
                      onClick={() => setShowRemovePassword((current) => !current)}
                    >
                      {showRemovePassword ? <EyeOff size={17} /> : <Eye size={17} />}
                    </button>
                  </div>
                </label>

                {firmwareError && <p className="credentials-error">{firmwareError}</p>}

                <div className="credentials-actions">
                  <button
                    className="secondary-button"
                    type="button"
                    onClick={() => {
                      setReceiverPendingRemoval(null)
                      setRemovePassword('')
                      setShowRemovePassword(false)
                    }}
                  >
                    Cancel
                  </button>
                  <button
                    className="danger-button"
                    type="button"
                    onClick={() => void removeReceiver()}
                    disabled={isSavingReceiver || !removePassword}
                  >
                    {isSavingReceiver ? <Loader2 className="spin" size={16} /> : <X size={16} />}
                    Remove Receiver
                  </button>
                </div>
              </div>
            </section>
          </div>,
          document.body,
        )}
    </section>
  )
}

function LoginScreen({
  username,
  password,
  showPassword,
  error,
  onUsernameChange,
  onPasswordChange,
  onTogglePassword,
  onSubmit,
}: {
  username: string
  password: string
  showPassword: boolean
  error: string
  onUsernameChange: (value: string) => void
  onPasswordChange: (value: string) => void
  onTogglePassword: () => void
  onSubmit: (event: FormEvent<HTMLFormElement>) => void
}) {
  return (
    <main className="login-shell">
      <div className="trimble-watermark" aria-hidden="true">
        <span>Trimble</span>
      </div>
      <div className="login-motion" aria-hidden="true">
        <span />
        <span />
        <span />
        <span />
      </div>

      <section className="login-panel" aria-label="Sign in">
        <div className="login-brand">
          <div className="brand-mark">
            <img src={qaCenterIcon} alt="" aria-hidden="true" />
          </div>
          <div>
            <span className="brand-title">QA Receiver Center</span>
            <span className="brand-subtitle">Trimble QA tools</span>
          </div>
        </div>

        <div className="login-heading">
          <p className="eyebrow">Secure workspace</p>
          <h1>Sign in</h1>
          <p>Access receiver diagnostics, firmware tools, reports, and fleet operations.</p>
        </div>

        <form className="login-form" onSubmit={onSubmit}>
          <label className="login-field">
            <span>Username</span>
            <div>
              <UserRound size={18} />
              <input
                autoComplete="username"
                value={username}
                onChange={(event) => onUsernameChange(event.target.value)}
                placeholder="Enter username"
              />
            </div>
          </label>

          <label className="login-field">
            <span>Password</span>
            <div>
              <Lock size={18} />
              <input
                autoComplete="current-password"
                type={showPassword ? 'text' : 'password'}
                value={password}
                onChange={(event) => onPasswordChange(event.target.value)}
                placeholder="Enter password"
              />
              <button type="button" className="password-toggle" onClick={onTogglePassword}>
                {showPassword ? <EyeOff size={17} /> : <Eye size={17} />}
              </button>
            </div>
          </label>

          {error && <p className="login-error">{error}</p>}

          <button className="login-button" type="submit">
            <ShieldCheck size={18} />
            Sign in
          </button>
        </form>

      </section>
    </main>
  )
}

function BootLoadingScreen() {
  return (
    <main className="boot-shell">
      <section className="boot-panel" aria-label="Loading receiver data">
        <div className="brand-mark">
          <img src={qaCenterIcon} alt="" aria-hidden="true" />
        </div>
        <Loader2 className="spin" size={42} />
        <h1>Preparing Dashboard</h1>
        <p>Loading receiver configuration and running the initial diagnostics scan.</p>
        <div className="boot-bars" aria-hidden="true">
          <span />
          <span />
          <span />
        </div>
      </section>
    </main>
  )
}

function DashboardOfflineList({
  receivers,
  onOpen,
  onShowLoginDetails,
}: {
  receivers: Receiver[]
  onOpen: (ip: string) => void | Promise<void>
  onShowLoginDetails: (receiver: Receiver) => void
}) {
  if (!receivers.length) {
    return (
      <div className="offline-empty">
        <div className="offline-empty-icon">
          <ShieldCheck size={28} />
        </div>
        <h3>No offline receivers</h3>
        <p>All scanned receivers are currently reachable.</p>
      </div>
    )
  }

  return (
    <div className="receiver-list dashboard-receiver-list">
      {receivers.map((receiver) => (
        <ReceiverRow
          receiver={receiver}
          onOpen={onOpen}
          onShowLoginDetails={onShowLoginDetails}
          key={receiver.ip}
        />
      ))}
    </div>
  )
}

function ReceiverActions({
  receiver,
  onOpen,
  onShowLoginDetails,
  tone = 'default',
}: {
  receiver: Receiver
  onOpen: (ip: string) => void | Promise<void>
  onShowLoginDetails: (receiver: Receiver) => void
  tone?: 'default' | 'danger'
}) {
  const [isOpen, setIsOpen] = useState(false)
  const [menuPosition, setMenuPosition] = useState<ActionMenuPosition | null>(null)
  const buttonRef = useRef<HTMLButtonElement>(null)
  const menuId = `receiver-menu-${receiver.ip.replace(/\W/g, '-')}`

  const updateMenuPosition = useCallback(() => {
    const button = buttonRef.current
    if (!button) {
      return
    }

    const rect = button.getBoundingClientRect()
    const left = Math.max(
      VIEWPORT_GUTTER,
      Math.min(rect.right - ACTION_MENU_WIDTH, window.innerWidth - ACTION_MENU_WIDTH - VIEWPORT_GUTTER),
    )
    const belowTop = rect.bottom + ACTION_MENU_GAP
    const aboveTop = rect.top - ACTION_MENU_HEIGHT - ACTION_MENU_GAP
    const top =
      belowTop + ACTION_MENU_HEIGHT > window.innerHeight - VIEWPORT_GUTTER
        ? Math.max(VIEWPORT_GUTTER, aboveTop)
        : belowTop

    setMenuPosition({ top, left })
  }, [])

  const toggleMenu = useCallback(() => {
    if (isOpen) {
      setIsOpen(false)
      return
    }

    updateMenuPosition()
    setIsOpen(true)
  }, [isOpen, updateMenuPosition])

  useEffect(() => {
    if (!isOpen) {
      return
    }

    const handlePositionChange = () => updateMenuPosition()
    window.addEventListener('resize', handlePositionChange)
    window.addEventListener('scroll', handlePositionChange, true)

    return () => {
      window.removeEventListener('resize', handlePositionChange)
      window.removeEventListener('scroll', handlePositionChange, true)
    }
  }, [isOpen, updateMenuPosition])

  return (
    <div className={`receiver-actions ${tone}`}>
      <button
        ref={buttonRef}
        className="receiver-action-button"
        type="button"
        aria-haspopup="menu"
        aria-expanded={isOpen}
        aria-controls={isOpen ? menuId : undefined}
        aria-label={`Open actions for ${receiver.receiverName || receiver.ip}`}
        onClick={toggleMenu}
      >
        <MoreHorizontal size={17} />
        <span>Manage</span>
      </button>

      {isOpen &&
        menuPosition &&
        createPortal(
          <>
            <button
              className="receiver-action-scrim"
              type="button"
              aria-label="Close receiver actions"
              onClick={() => setIsOpen(false)}
            />
            <div
              className="receiver-action-menu"
              id={menuId}
              role="menu"
              style={{ top: menuPosition.top, left: menuPosition.left }}
            >
              <button
                type="button"
                role="menuitem"
                onClick={() => {
                  setIsOpen(false)
                  void onOpen(receiver.ip)
                }}
              >
                <ExternalLink size={16} />
                <span>Open Web Interface</span>
              </button>
              <button type="button" role="menuitem" disabled>
                <ClipboardList size={16} />
                <span>Detailed Report</span>
              </button>
              <button type="button" role="menuitem" disabled>
                <Activity size={16} />
                <span>Diagnostics</span>
              </button>
              <button
                type="button"
                role="menuitem"
                onClick={() => {
                  setIsOpen(false)
                  onShowLoginDetails(receiver)
                }}
              >
                <KeyRound size={16} />
                <span>Login Details</span>
              </button>
            </div>
          </>,
          document.body,
        )}
    </div>
  )
}

function ReceiverRow({
  receiver,
  onOpen,
  onShowLoginDetails,
}: {
  receiver: Receiver
  onOpen: (ip: string) => void | Promise<void>
  onShowLoginDetails: (receiver: Receiver) => void
}) {
  const reviewClass = receiver.needsReview ? 'review' : receiver.status
  const issueLabel = receiver.status === 'offline' ? 'Offline' : receiver.needsReview ? 'Needs review' : 'Healthy'

  return (
    <article className={`receiver-row receiver-card-row ${reviewClass}`} title={receiver.lastError ?? `Receiver ${receiver.ip}`}>
      <div className="receiver-card-status">
        <div className={`status-dot ${reviewClass}`} aria-label={issueLabel} />
      </div>
      <div className="receiver-card-main">
        <div className="receiver-card-title">
          <div className="receiver-main">
            <strong>{receiver.receiverName || receiver.ip}</strong>
            <span>
              {receiver.ip} - Group {receiver.group}
              {receiver.firmwareDate ? ` - ${receiver.firmwareDate}` : ''}
            </span>
          </div>
          <span className={`status-pill ${receiver.status}`}>{statusLabels[receiver.status]}</span>
        </div>
        <div className="receiver-card-stats">
          <span className="firmware-mini-stat">
            <Gauge size={14} />
            FW {receiver.firmwareVersion || '--'}
          </span>
          <span className="firmware-mini-stat">
            <Cpu size={14} />
            {formatNumber(receiver.celsius, 'C')}
          </span>
          <span className="firmware-mini-stat target">
            <Activity size={14} />
            {receiver.fixType || '--'}
          </span>
          <span className="firmware-mini-stat">
            <BarChart3 size={14} />
            SV {formatNumber(receiver.svSolved)}
          </span>
          <span className={`firmware-mini-stat ${receiver.needsReview ? 'warning' : ''}`}>
            <AlertTriangle size={14} />
            ERR {formatNumber(receiver.errorWarning)}
          </span>
        </div>
        {receiver.lastError && receiver.status !== 'offline' && <p className="receiver-card-error">{receiver.lastError}</p>}
      </div>
      <ReceiverActions receiver={receiver} onOpen={onOpen} onShowLoginDetails={onShowLoginDetails} />
    </article>
  )
}

function HistoryPage() {
  const [activeHistoryMode, setActiveHistoryMode] = useState<'single' | 'multiple' | 'groups'>('single')
  const [selectedGroup, setSelectedGroup] = useState('Alpha')
  const [history, setHistory] = useState<FirmwareSingleUpdateHistoryEntry[]>([])
  const [isLoadingHistory, setIsLoadingHistory] = useState(false)
  const [historyError, setHistoryError] = useState('')
  const [cloneDownloadToast, setCloneDownloadToast] = useState('')
  const [historyQuery, setHistoryQuery] = useState('')
  const [historyResultFilter, setHistoryResultFilter] = useState('all')
  const [selectedHistoryDate, setSelectedHistoryDate] = useState<string | null>(null)
  const [calendarMonth, setCalendarMonth] = useState(() => startOfLocalMonth(new Date()))
  const [isCalendarOpen, setIsCalendarOpen] = useState(false)
  const [historyPendingDeletion, setHistoryPendingDeletion] = useState<FirmwareSingleUpdateHistoryEntry | null>(null)
  const [historyDeletePassword, setHistoryDeletePassword] = useState('')
  const [showHistoryDeletePassword, setShowHistoryDeletePassword] = useState(false)
  const [isDeletingHistory, setIsDeletingHistory] = useState(false)
  const [historyDeleteError, setHistoryDeleteError] = useState('')
  const historyCalendarRef = useRef<HTMLDivElement>(null)

  const loadHistory = useCallback(async () => {
    try {
      setIsLoadingHistory(true)
      setHistoryError('')
      const entries = await invoke<FirmwareSingleUpdateHistoryEntry[]>('get_single_firmware_update_history')
      setHistory(entries)
    } catch (error) {
      setHistoryError(formatError(error))
    } finally {
      setIsLoadingHistory(false)
    }
  }, [])

  useEffect(() => {
    const timer = window.setTimeout(() => {
      void loadHistory()
    }, 0)

    return () => window.clearTimeout(timer)
  }, [loadHistory])

  useEffect(() => {
    if (!cloneDownloadToast) {
      return undefined
    }

    const timer = window.setTimeout(() => {
      setCloneDownloadToast('')
    }, 2800)

    return () => window.clearTimeout(timer)
  }, [cloneDownloadToast])

  useEffect(() => {
    if (!isCalendarOpen) {
      return undefined
    }

    const closeCalendarOnOutsideClick = (event: PointerEvent) => {
      if (!historyCalendarRef.current?.contains(event.target as Node)) {
        setIsCalendarOpen(false)
      }
    }

    document.addEventListener('pointerdown', closeCalendarOnOutsideClick)
    return () => document.removeEventListener('pointerdown', closeCalendarOnOutsideClick)
  }, [isCalendarOpen])

  const downloadCloneFile = useCallback(async (path: string | null) => {
    if (!path) {
      return
    }

    try {
      setHistoryError('')
      setCloneDownloadToast('')
      const result = await invoke<CloneFileDownloadResult>('download_clone_file_backup', { path })
      setCloneDownloadToast(`Downloaded ${result.fileName}`)
    } catch (error) {
      setHistoryError(formatError(error))
    }
  }, [])

  const deleteHistoryEntry = useCallback(async () => {
    if (!historyPendingDeletion) {
      return
    }

    try {
      setIsDeletingHistory(true)
      setHistoryDeleteError('')
      const entries = await invoke<FirmwareSingleUpdateHistoryEntry[]>('delete_single_firmware_update_history', {
        jobId: historyPendingDeletion.jobId,
        appPassword: historyDeletePassword,
      })
      setHistory(entries)
      setHistoryPendingDeletion(null)
      setHistoryDeletePassword('')
      setShowHistoryDeletePassword(false)
      setCloneDownloadToast('History entry and configuration backup deleted')
    } catch (error) {
      setHistoryDeleteError(formatError(error))
    } finally {
      setIsDeletingHistory(false)
    }
  }, [historyDeletePassword, historyPendingDeletion])

  const scopedHistory = useMemo(
    () =>
      history.filter((entry) => {
        const operationType = historyOperationType(entry)
        if (activeHistoryMode === 'single') {
          return operationType === 'single'
        }
        if (activeHistoryMode === 'multiple') {
          return operationType === 'multiple'
        }
        return operationType === 'groups' && (!entry.groupName || entry.groupName === selectedGroup)
      }),
    [activeHistoryMode, history, selectedGroup],
  )

  const historyDateCounts = useMemo(() => {
    const counts = new Map<string, number>()
    scopedHistory.forEach((entry) => {
      const key = historyDateKey(entry.startedAt)
      if (key) {
        counts.set(key, (counts.get(key) || 0) + 1)
      }
    })
    return counts
  }, [scopedHistory])

  const filteredHistory = useMemo(() => {
    const needle = historyQuery.trim().toLowerCase()
    return scopedHistory.filter((entry) => {
      const matchesQuery =
        !needle ||
        [entry.ip, entry.receiverName, entry.model, entry.targetFile, entry.finalFirmwareVersion]
          .filter(Boolean)
          .join(' ')
          .toLowerCase()
          .includes(needle)
      const matchesResult =
        historyResultFilter === 'all' || entry.result.toLowerCase() === historyResultFilter.toLowerCase()
      const matchesDate = !selectedHistoryDate || historyDateKey(entry.startedAt) === selectedHistoryDate
      return matchesQuery && matchesResult && matchesDate
    })
  }, [historyQuery, historyResultFilter, scopedHistory, selectedHistoryDate])

  const calendarDays = useMemo(() => buildHistoryCalendarDays(calendarMonth), [calendarMonth])
  const completedCount = scopedHistory.filter((entry) => entry.result === 'Completed').length
  const failedCount = scopedHistory.filter((entry) => entry.result === 'Failed').length
  const groupOptions = [
    { value: 'Alpha', label: 'Alpha Receivers' },
    { value: 'Beta', label: 'Beta Receivers' },
  ]

  return (
    <section className="surface history-surface">
      <header className="history-header">
        <div className="firmware-title">
          <div className="firmware-icon">
            <History size={22} />
          </div>
          <div>
            <h2>Firmware Update History</h2>
            <p>Audit trail for receiver firmware update operations.</p>
          </div>
        </div>
        <button className="secondary-button" type="button" onClick={() => void loadHistory()} disabled={isLoadingHistory}>
          {isLoadingHistory ? <Loader2 className="spin" size={16} /> : <RefreshCw size={16} />}
          Refresh
        </button>
      </header>

      <div className="history-metrics">
        <div>
          <span>Total Updates</span>
          <strong>{scopedHistory.length}</strong>
        </div>
        <div>
          <span>Completed</span>
          <strong>{completedCount}</strong>
        </div>
        <div>
          <span>Failed</span>
          <strong>{failedCount}</strong>
        </div>
      </div>

      <section className="history-module">
        <div className="history-module-header">
          <div>
            <h3>Firmware Update</h3>
            <p>Browse update history by update type and receiver group.</p>
          </div>
          {activeHistoryMode === 'groups' && (
            <div className="history-group-filter">
              <span>Group</span>
              <select value={selectedGroup} onChange={(event) => setSelectedGroup(event.target.value)}>
                {groupOptions.map((group) => (
                  <option value={group.value} key={group.value}>
                    {group.label}
                  </option>
                ))}
              </select>
            </div>
          )}
        </div>

        <div className="history-segmented" aria-label="Firmware update history mode">
          <button
            type="button"
            className={activeHistoryMode === 'single' ? 'active' : ''}
            onClick={() => setActiveHistoryMode('single')}
          >
            Single Receiver
          </button>
          <button
            type="button"
            className={activeHistoryMode === 'multiple' ? 'active' : ''}
            onClick={() => setActiveHistoryMode('multiple')}
          >
            Multiple Receiver
          </button>
          <button
            type="button"
            className={activeHistoryMode === 'groups' ? 'active' : ''}
            onClick={() => setActiveHistoryMode('groups')}
          >
            Groups
          </button>
        </div>

        <div className="history-filter-bar">
            <label className="history-search">
              <Search size={17} />
              <input
                value={historyQuery}
                onChange={(event) => setHistoryQuery(event.target.value)}
                placeholder="Search IP, receiver, model or firmware"
              />
            </label>

            <select
              className="history-result-filter"
              value={historyResultFilter}
              onChange={(event) => setHistoryResultFilter(event.target.value)}
              aria-label="Filter history by result"
            >
              <option value="all">All results</option>
              <option value="completed">Completed</option>
              <option value="failed">Failed</option>
            </select>

            <div className="history-calendar-control" ref={historyCalendarRef}>
              <button
                className={`secondary-button history-calendar-trigger ${selectedHistoryDate ? 'active' : ''}`}
                type="button"
                onClick={() => setIsCalendarOpen((current) => !current)}
              >
                <CalendarDays size={17} />
                {selectedHistoryDate ? formatSelectedHistoryDate(selectedHistoryDate) : 'Select date'}
              </button>

              {isCalendarOpen && (
                <div className="history-calendar-popover">
                  <div className="history-calendar-header">
                    <button
                      type="button"
                      aria-label="Previous month"
                      onClick={() => setCalendarMonth((current) => shiftLocalMonth(current, -1))}
                    >
                      <ChevronLeft size={17} />
                    </button>
                    <strong>
                      {calendarMonth.toLocaleDateString('en-GB', { month: 'long', year: 'numeric' })}
                    </strong>
                    <button
                      type="button"
                      aria-label="Next month"
                      onClick={() => setCalendarMonth((current) => shiftLocalMonth(current, 1))}
                    >
                      <ChevronRight size={17} />
                    </button>
                  </div>
                  <div className="history-calendar-weekdays">
                    {['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'].map((day) => (
                      <span key={day}>{day}</span>
                    ))}
                  </div>
                  <div className="history-calendar-grid">
                    {calendarDays.map((day) => {
                      const key = localDateKey(day)
                      const count = historyDateCounts.get(key) || 0
                      const outsideMonth = day.getMonth() !== calendarMonth.getMonth()
                      return (
                        <button
                          type="button"
                          key={key}
                          className={`${outsideMonth ? 'outside' : ''} ${
                            selectedHistoryDate === key ? 'selected' : ''
                          } ${count ? 'has-updates' : ''}`}
                          onClick={() => {
                            setSelectedHistoryDate((current) => (current === key ? null : key))
                            setIsCalendarOpen(false)
                          }}
                        >
                          <span>{day.getDate()}</span>
                          {count > 0 && <small>{count}</small>}
                        </button>
                      )
                    })}
                  </div>
                  <button
                    className="history-calendar-clear"
                    type="button"
                    onClick={() => {
                      setSelectedHistoryDate(null)
                      setIsCalendarOpen(false)
                    }}
                    disabled={!selectedHistoryDate}
                  >
                    Show all dates
                  </button>
                </div>
              )}
            </div>
        </div>

        {historyError && <p className="credentials-error">{historyError}</p>}
        {filteredHistory.length ? (
          <div className="history-list">
            {filteredHistory.map((entry) => (
              <article className={`history-entry ${entry.result.toLowerCase()}`} key={entry.jobId}>
                <div className="history-entry-main">
                  <strong>{entry.receiverName || entry.ip}</strong>
                  {activeHistoryMode !== 'single' && (
                    <small className="history-entry-context">
                      {activeHistoryMode === 'groups'
                        ? `${entry.groupName || selectedGroup} Receivers`
                        : 'Multiple Receiver Update'}
                      {entry.batchId ? ` - ${entry.batchId}` : ''}
                    </small>
                  )}
                  <span>
                    {entry.ip} - {entry.targetFile}
                  </span>
                  <small>
                    {formatHistoryTime(entry.startedAt)} to {formatHistoryTime(entry.completedAt)}
                  </small>
                  {entry.cloneFileName && (
                    <button
                      className="history-clone-file"
                      type="button"
                      onClick={() => void downloadCloneFile(entry.cloneFilePath)}
                      disabled={!entry.cloneFilePath}
                    >
                      <span>Configuration backup</span>
                      <strong>{entry.cloneFileName}</strong>
                    </button>
                  )}
                  {entry.errors.length > 0 && <p>{entry.errors[0]}</p>}
                </div>
                <div className="history-entry-side">
                  <div className="history-entry-actions">
                    <span className={`preflight-pill ${entry.result.toLowerCase()}`}>{entry.result}</span>
                    <button
                      className="history-delete-button"
                      type="button"
                      aria-label={`Delete history entry for ${entry.ip}`}
                      onClick={() => {
                        setHistoryPendingDeletion(entry)
                        setHistoryDeletePassword('')
                        setHistoryDeleteError('')
                        setShowHistoryDeletePassword(false)
                      }}
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                  <small>Final FW: {entry.finalFirmwareVersion || '--'}</small>
                </div>
              </article>
            ))}
          </div>
        ) : (
          <div className="history-empty">
            <ClipboardList size={34} />
            <strong>
              {scopedHistory.length
                ? 'No updates match the selected filters'
                : activeHistoryMode === 'single'
                  ? 'No single receiver updates recorded yet'
                  : activeHistoryMode === 'multiple'
                    ? 'No multiple receiver updates recorded yet'
                    : `No ${selectedGroup} group updates recorded yet`}
            </strong>
            <p>
              {scopedHistory.length
                ? 'Change the search, result filter, or selected calendar date.'
                : 'Completed firmware updates will appear here automatically.'}
            </p>
          </div>
        )}
      </section>

      {cloneDownloadToast && <div className="history-download-toast">{cloneDownloadToast}</div>}

      {historyPendingDeletion &&
        createPortal(
          <div className="modal-layer">
            <button
              className="modal-backdrop"
              type="button"
              aria-label="Close delete history dialog"
              onClick={() => {
                setHistoryPendingDeletion(null)
                setHistoryDeletePassword('')
                setHistoryDeleteError('')
              }}
            />
            <section className="firmware-confirm-modal" aria-label="Delete firmware update history entry">
              <header className="credentials-header danger">
                <div className="credentials-title">
                  <div className="credentials-icon danger">
                    <Trash2 size={20} />
                  </div>
                  <div>
                    <h2>Delete History Entry</h2>
                    <p>This also permanently deletes the associated configuration backup.</p>
                  </div>
                </div>
                <button
                  className="modal-close"
                  type="button"
                  aria-label="Close delete history dialog"
                  onClick={() => setHistoryPendingDeletion(null)}
                >
                  <X size={18} />
                </button>
              </header>

              <form
                className="credentials-form"
                onSubmit={(event) => {
                  event.preventDefault()
                  void deleteHistoryEntry()
                }}
              >
                <div className="history-delete-summary">
                  <strong>{historyPendingDeletion.receiverName || historyPendingDeletion.ip}</strong>
                  <span>
                    {historyPendingDeletion.ip} - {historyPendingDeletion.targetFile}
                  </span>
                  {historyPendingDeletion.cloneFileName && (
                    <small>Backup: {historyPendingDeletion.cloneFileName}</small>
                  )}
                </div>

                <label className="credentials-field">
                  <span>Application Password</span>
                  <div>
                    <Lock size={17} />
                    <input
                      autoFocus
                      autoComplete="current-password"
                      type={showHistoryDeletePassword ? 'text' : 'password'}
                      value={historyDeletePassword}
                      onChange={(event) => setHistoryDeletePassword(event.target.value)}
                      placeholder="Enter your current password"
                    />
                    <button
                      type="button"
                      className="password-toggle"
                      onClick={() => setShowHistoryDeletePassword((current) => !current)}
                    >
                      {showHistoryDeletePassword ? <EyeOff size={17} /> : <Eye size={17} />}
                    </button>
                  </div>
                </label>

                {historyDeleteError && <p className="credentials-error">{historyDeleteError}</p>}
                <div className="credentials-actions">
                  <button
                    className="secondary-button"
                    type="button"
                    onClick={() => setHistoryPendingDeletion(null)}
                  >
                    Cancel
                  </button>
                  <button
                    className="danger-button"
                    type="submit"
                    disabled={!historyDeletePassword || isDeletingHistory}
                  >
                    {isDeletingHistory ? <Loader2 className="spin" size={16} /> : <Trash2 size={16} />}
                    Delete Entry
                  </button>
                </div>
              </form>
            </section>
          </div>,
          document.body,
        )}
    </section>
  )
}

const DEFAULT_UPDATE_SAFETY_SETTINGS: UpdateSafetySettings = {
  postRestartDelayMinutes: 2,
  preRestartSafetyMarginMinutes: 10,
  restartCycleMinutes: 30,
}

function UpdateSafetyPage({ onActivity }: { onActivity: (message: string) => void }) {
  const [settings, setSettings] = useState<UpdateSafetySettings>(DEFAULT_UPDATE_SAFETY_SETTINGS)
  const [savedSettings, setSavedSettings] = useState<UpdateSafetySettings>(DEFAULT_UPDATE_SAFETY_SETTINGS)
  const [isLoading, setIsLoading] = useState(true)
  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState('')
  const [savedMessage, setSavedMessage] = useState('')

  const loadSettings = useCallback(async () => {
    try {
      setIsLoading(true)
      setError('')
      const loaded = isTauri()
        ? await invoke<UpdateSafetySettings>('get_update_safety_settings')
        : DEFAULT_UPDATE_SAFETY_SETTINGS
      setSettings(loaded)
      setSavedSettings(loaded)
    } catch (loadError) {
      setError(formatError(loadError))
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => {
    const timer = window.setTimeout(() => {
      void loadSettings()
    }, 0)
    return () => window.clearTimeout(timer)
  }, [loadSettings])

  useEffect(() => {
    if (!savedMessage) {
      return undefined
    }
    const timer = window.setTimeout(() => setSavedMessage(''), 2800)
    return () => window.clearTimeout(timer)
  }, [savedMessage])

  const safeWindowEnd = settings.restartCycleMinutes - settings.preRestartSafetyMarginMinutes
  const hasValidWindow =
    settings.postRestartDelayMinutes >= 0 &&
    settings.preRestartSafetyMarginMinutes >= 0 &&
    settings.postRestartDelayMinutes < settings.restartCycleMinutes &&
    settings.preRestartSafetyMarginMinutes < settings.restartCycleMinutes &&
    settings.postRestartDelayMinutes + settings.preRestartSafetyMarginMinutes < settings.restartCycleMinutes
  const hasChanges = JSON.stringify(settings) !== JSON.stringify(savedSettings)
  const postRestartWidth = `${(settings.postRestartDelayMinutes / settings.restartCycleMinutes) * 100}%`
  const safeWindowWidth = `${
    (Math.max(0, safeWindowEnd - settings.postRestartDelayMinutes) / settings.restartCycleMinutes) * 100
  }%`
  const preRestartWidth = `${(settings.preRestartSafetyMarginMinutes / settings.restartCycleMinutes) * 100}%`

  const updateMinutes = (field: 'postRestartDelayMinutes' | 'preRestartSafetyMarginMinutes', value: string) => {
    const parsed = Number.parseInt(value, 10)
    setSavedMessage('')
    setSettings((current) => ({
      ...current,
      [field]: Number.isFinite(parsed) ? Math.max(0, Math.min(29, parsed)) : 0,
    }))
  }

  const saveSettings = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!hasValidWindow || isSaving) {
      return
    }

    try {
      setIsSaving(true)
      setError('')
      const saved = isTauri()
        ? await invoke<UpdateSafetySettings>('save_update_safety_settings', { settings })
        : settings
      setSettings(saved)
      setSavedSettings(saved)
      setSavedMessage('Update Safety settings saved')
      onActivity(
        `Update Safety changed: ${saved.postRestartDelayMinutes}m after restart, ${saved.preRestartSafetyMarginMinutes}m before scheduled restart`,
      )
    } catch (saveError) {
      setError(formatError(saveError))
    } finally {
      setIsSaving(false)
    }
  }

  return (
    <section className="surface update-safety-surface">
      <header className="update-safety-header">
          <div className="firmware-title">
            <div className="firmware-icon">
              <ShieldCheck size={22} />
            </div>
            <div>
              <h2>Update Safety</h2>
            </div>
          </div>
      </header>

      <form className="update-safety-layout" onSubmit={saveSettings}>
        <section className="update-safety-editor" aria-labelledby="update-window-title">
          <div className="update-safety-section-heading">
            <div>
              <h3 id="update-window-title">Convergence Update Window</h3>
              <p>The 30-minute receiver cycle remains fixed. Both safety margins are configurable.</p>
            </div>
            <div className="update-safety-cycle">
              <Clock3 size={18} />
              <span>Restart cycle</span>
              <strong>{settings.restartCycleMinutes} min</strong>
            </div>
          </div>

          <div className="update-safety-fields">
            <label className="update-safety-field">
              <span>Delay after receiver restart</span>
              <div>
                <input
                  type="number"
                  min="0"
                  max="29"
                  step="1"
                  value={settings.postRestartDelayMinutes}
                  onChange={(event) => updateMinutes('postRestartDelayMinutes', event.target.value)}
                  disabled={isLoading || isSaving}
                />
                <span>minutes</span>
              </div>
            </label>

            <label className="update-safety-field">
              <span>Margin before scheduled restart</span>
              <div>
                <input
                  type="number"
                  min="0"
                  max="29"
                  step="1"
                  value={settings.preRestartSafetyMarginMinutes}
                  onChange={(event) => updateMinutes('preRestartSafetyMarginMinutes', event.target.value)}
                  disabled={isLoading || isSaving}
                />
                <span>minutes</span>
              </div>
            </label>
          </div>

          <div className="update-window-preview" aria-label="Configured update window">
            <div className="update-window-labels">
              <span>Restart</span>
              <strong>
                {hasValidWindow
                  ? `Allowed after ${settings.postRestartDelayMinutes}:00 and before ${safeWindowEnd}:00`
                  : 'Invalid window'}
              </strong>
              <span>30:00</span>
            </div>
            <div className="update-window-track" aria-hidden="true">
              <span className="blocked-start" style={{ width: postRestartWidth }} />
              <span className="allowed" style={{ width: safeWindowWidth }} />
              <span className="blocked-end" style={{ width: preRestartWidth }} />
            </div>
            <div className="update-window-legend">
              <span><i className="blocked" />Blocked</span>
              <span><i className="allowed" />Update allowed</span>
            </div>
          </div>

          {!hasValidWindow && (
            <p className="firmware-inline-alert" role="alert">
              <AlertTriangle size={17} />
              <span>The two margins must leave time for an update between them.</span>
            </p>
          )}
          {error && <p className="credentials-error">{error}</p>}
        </section>

        <footer className="update-safety-actions">
          <span>{hasChanges ? 'Unsaved changes' : 'Settings are up to date'}</span>
          <div>
            <button
              className="secondary-button"
              type="button"
              onClick={() => {
                setSettings(DEFAULT_UPDATE_SAFETY_SETTINGS)
                setSavedMessage('')
                setError('')
              }}
              disabled={isLoading || isSaving}
            >
              Restore Defaults
            </button>
            <button className="secondary-button" type="button" onClick={() => void loadSettings()} disabled={isLoading || isSaving}>
              {isLoading ? <Loader2 className="spin" size={16} /> : <RefreshCw size={16} />}
              Reload
            </button>
            <button className="primary-button" type="submit" disabled={!hasValidWindow || !hasChanges || isLoading || isSaving}>
              {isSaving ? <Loader2 className="spin" size={16} /> : <Save size={16} />}
              Save Settings
            </button>
          </div>
        </footer>
      </form>

      {savedMessage && <div className="history-download-toast">{savedMessage}</div>}
    </section>
  )
}

function LoginDetailsModal({
  receiver,
  password,
  showPassword,
  details,
  error,
  isLoading,
  showReceiverPassword,
  onPasswordChange,
  onTogglePassword,
  onToggleReceiverPassword,
  onSubmit,
  onClose,
}: {
  receiver: Receiver
  password: string
  showPassword: boolean
  details: ReceiverLoginDetails | null
  error: string
  isLoading: boolean
  showReceiverPassword: boolean
  onPasswordChange: (value: string) => void
  onTogglePassword: () => void
  onToggleReceiverPassword: () => void
  onSubmit: (event: FormEvent<HTMLFormElement>) => void
  onClose: () => void
}) {
  const displayPassword = details ? (showReceiverPassword ? details.password : '*'.repeat(12)) : ''

  return createPortal(
    <div className="modal-layer" role="presentation">
      <button className="modal-backdrop" type="button" aria-label="Close login details" onClick={onClose} />
      <section className="credentials-modal" role="dialog" aria-modal="true" aria-labelledby="credentials-title">
        <header className="credentials-header">
          <div className="credentials-title">
            <div className="credentials-icon">
              <KeyRound size={20} />
            </div>
            <div>
              <h2 id="credentials-title">Login Details</h2>
              <p>
                {receiver.receiverName || receiver.ip} - {receiver.ip}
              </p>
            </div>
          </div>
          <button className="modal-close" type="button" aria-label="Close login details" onClick={onClose}>
            <X size={18} />
          </button>
        </header>

        {!details ? (
          <form className="credentials-form" onSubmit={onSubmit}>
            <p className="credentials-note">
              Re-enter your application password to reveal receiver credentials from the configuration file.
            </p>
            <label className="credentials-field">
              <span>Application Password</span>
              <div>
                <Lock size={17} />
                <input
                  autoFocus
                  type={showPassword ? 'text' : 'password'}
                  value={password}
                  onChange={(event) => onPasswordChange(event.target.value)}
                  placeholder="Enter your password"
                />
                <button type="button" className="password-toggle" onClick={onTogglePassword}>
                  {showPassword ? <EyeOff size={17} /> : <Eye size={17} />}
                </button>
              </div>
            </label>
            {error && <p className="credentials-error">{error}</p>}
            <div className="credentials-actions">
              <button className="secondary-button" type="button" onClick={onClose}>
                Cancel
              </button>
              <button className="primary-button" type="submit" disabled={isLoading || !password}>
                {isLoading ? <Loader2 className="spin" size={16} /> : <ShieldCheck size={16} />}
                Verify
              </button>
            </div>
          </form>
        ) : (
          <div className="credential-result">
            <div className="credential-row">
              <span>Receiver IP</span>
              <strong>{details.ip}</strong>
            </div>
            <div className="credential-row">
              <span>Username</span>
              <strong>{details.user}</strong>
            </div>
            <div className="credential-row secret">
              <span>Password</span>
              <strong>{displayPassword}</strong>
              <button type="button" className="secret-toggle" onClick={onToggleReceiverPassword}>
                {showReceiverPassword ? <EyeOff size={16} /> : <Eye size={16} />}
                {showReceiverPassword ? 'Hide' : 'Show'}
              </button>
            </div>
            <button
              className="copy-button"
              type="button"
              onClick={() => void navigator.clipboard?.writeText(`${details.user}\n${details.password}`)}
            >
              <Copy size={16} />
              Copy Credentials
            </button>
          </div>
        )}
      </section>
    </div>,
    document.body,
  )
}

function MetricTile({
  icon: Icon,
  tone,
  label,
  value,
}: {
  icon: typeof Radio
  tone: 'success' | 'warning' | 'neutral' | 'danger'
  label: string
  value: number
}) {
  return (
    <article className="metric-tile">
      <div className={`metric-icon ${tone}`}>
        <Icon size={20} />
      </div>
      <div>
        <span>{label}</span>
        <strong>{value}</strong>
      </div>
    </article>
  )
}

function formatNumber(value: number | null, suffix = '') {
  if (value === null || Number.isNaN(value)) {
    return '--'
  }

  return `${Number.isInteger(value) ? value : value.toFixed(1)}${suffix}`
}

function formatDuration(value: number | null | undefined) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return '--'
  }

  const totalSeconds = Math.max(0, Math.floor(value))
  const days = Math.floor(totalSeconds / 86400)
  const hours = Math.floor((totalSeconds % 86400) / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)

  if (days > 0) {
    return `${days}d ${hours}h`
  }

  if (hours > 0) {
    return `${hours}h ${minutes}m`
  }

  return `${minutes}m`
}

function formatBytes(value: number) {
  if (!Number.isFinite(value)) {
    return '--'
  }

  const units = ['B', 'KB', 'MB', 'GB']
  let size = value
  let unitIndex = 0
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex += 1
  }

  return `${size >= 10 ? size.toFixed(0) : size.toFixed(1)} ${units[unitIndex]}`
}

function formatProgressPercent(value: number) {
  if (!Number.isFinite(value)) {
    return '--'
  }

  const safeValue = Math.max(0, Math.min(100, value))
  return `${safeValue >= 10 ? safeValue.toFixed(0) : safeValue.toFixed(1)}%`
}

function getFirmwareJobProgress(job: FirmwareSingleUpdateJobStatus) {
  return job.status === 'Completed' ? 100 : Math.max(0, Math.min(100, job.updatePercent ?? 0))
}

function isFirmwareWorkflowMilestone(phase: string) {
  return [
    'ReceiverRestart',
    'ReceiverOnline',
    'RestorePassword',
    'VerifyResult',
    'Completed',
    'Cancelled',
    'Failed',
  ].includes(phase)
}

function getFirmwareJobDisplayState(job: FirmwareSingleUpdateJobStatus) {
  if (isFirmwareWorkflowMilestone(job.phase)) {
    return formatWorkflowState(job.phase)
  }

  return job.updateMode || job.fwStatus || formatWorkflowState(job.phase)
}

function waitForUi(milliseconds: number) {
  return new Promise<void>((resolve) => {
    window.setTimeout(resolve, milliseconds)
  })
}

function startOfLocalMonth(value: Date) {
  return new Date(value.getFullYear(), value.getMonth(), 1)
}

function shiftLocalMonth(value: Date, amount: number) {
  return new Date(value.getFullYear(), value.getMonth() + amount, 1)
}

function localDateKey(value: Date) {
  const year = value.getFullYear()
  const month = String(value.getMonth() + 1).padStart(2, '0')
  const day = String(value.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

function historyOperationType(entry: FirmwareSingleUpdateHistoryEntry): 'single' | 'multiple' | 'groups' {
  const explicitType = entry.operationType?.toLowerCase()
  if (explicitType === 'group') {
    return 'groups'
  }
  if (explicitType === 'multiple') {
    return 'multiple'
  }

  if (entry.batchId?.startsWith('group-fw-')) {
    return 'groups'
  }
  if (entry.batchId?.startsWith('multiple-fw-')) {
    return 'multiple'
  }
  return 'single'
}

function historyDateKey(value: string | null) {
  if (!value) {
    return null
  }

  const seconds = Number(value)
  if (!Number.isFinite(seconds)) {
    return null
  }

  return localDateKey(new Date(seconds * 1000))
}

function buildHistoryCalendarDays(month: Date) {
  const firstDay = startOfLocalMonth(month)
  const mondayOffset = (firstDay.getDay() + 6) % 7
  const firstVisibleDay = new Date(firstDay.getFullYear(), firstDay.getMonth(), 1 - mondayOffset)
  return Array.from(
    { length: 42 },
    (_, index) =>
      new Date(firstVisibleDay.getFullYear(), firstVisibleDay.getMonth(), firstVisibleDay.getDate() + index),
  )
}

function formatSelectedHistoryDate(value: string) {
  const [year, month, day] = value.split('-').map(Number)
  if (!year || !month || !day) {
    return value
  }

  return new Date(year, month - 1, day).toLocaleDateString('en-GB', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
  })
}

function formatHistoryTime(value: string | null) {
  if (!value) {
    return '--'
  }

  const seconds = Number(value)
  if (!Number.isFinite(seconds)) {
    return value
  }

  return new Date(seconds * 1000).toLocaleString('en-GB', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}

function formatWorkflowState(value: string) {
  return value
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .replace(/Package Check/i, 'Package Check')
}

function formatError(error: unknown) {
  return error instanceof Error ? error.message : String(error)
}

export default App
