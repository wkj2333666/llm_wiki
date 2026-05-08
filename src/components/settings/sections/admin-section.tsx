import { useEffect, useState, useCallback } from "react"
import { useTranslation } from "react-i18next"
import { UserPlus, Pencil, Trash2, Loader2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog"
import * as adminApi from "@/api/admin"
import { listProjects } from "@/api/project"
import type { ProjectInfo } from "@/api/project"
import { useAuthStore } from "@/stores/auth-store"

type AdminTab = "users" | "projects"

export function AdminSection() {
  const { t } = useTranslation()
  const currentUser = useAuthStore((s) => s.user)
  const [tab, setTab] = useState<AdminTab>("users")

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold">{t("settings.sections.admin.title", "Admin")}</h2>
        <p className="text-sm text-muted-foreground">{t("settings.sections.admin.description", "Manage users and projects.")}</p>
      </div>

      <div className="flex gap-2 border-b">
        <button
          className={`px-3 py-2 text-sm font-medium border-b-2 transition-colors ${
            tab === "users" ? "border-primary text-primary" : "border-transparent text-muted-foreground hover:text-foreground"
          }`}
          onClick={() => setTab("users")}
        >
          {t("settings.sections.admin.tabs.users", "Users")}
        </button>
        <button
          className={`px-3 py-2 text-sm font-medium border-b-2 transition-colors ${
            tab === "projects" ? "border-primary text-primary" : "border-transparent text-muted-foreground hover:text-foreground"
          }`}
          onClick={() => setTab("projects")}
        >
          {t("settings.sections.admin.tabs.projects", "Projects")}
        </button>
      </div>

      {tab === "users" ? <UsersTab currentUser={currentUser} /> : <ProjectsTab />}
    </div>
  )
}

// ── Users Tab ─────────────────────────────────────────────────────

function UsersTab({ currentUser }: { currentUser: { username: string } | null }) {
  const { t } = useTranslation()
  const [users, setUsers] = useState<adminApi.User[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState("")
  const [addOpen, setAddOpen] = useState(false)
  const [editUser, setEditUser] = useState<adminApi.User | null>(null)

  const fetchUsers = useCallback(async () => {
    try {
      setError("")
      const list = await adminApi.listUsers()
      setUsers(list)
    } catch (err) {
      setError(String(err))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { fetchUsers() }, [fetchUsers])

  if (loading) {
    return <div className="flex items-center justify-center py-8"><Loader2 className="h-6 w-6 animate-spin" /></div>
  }

  return (
    <div className="space-y-4">
      {error && <p className="text-sm text-destructive">{error}</p>}

      <div className="flex justify-end">
        <Button size="sm" onClick={() => setAddOpen(true)}>
          <UserPlus className="mr-2 h-4 w-4" />
          {t("settings.sections.admin.users.addUser", "Add User")}
        </Button>
      </div>

      <div className="rounded-lg border overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="text-left px-4 py-2 font-medium">{t("settings.sections.admin.users.username", "Username")}</th>
              <th className="text-left px-4 py-2 font-medium">{t("settings.sections.admin.users.role", "Role")}</th>
              <th className="text-left px-4 py-2 font-medium">{t("settings.sections.admin.users.createdAt", "Created")}</th>
              <th className="text-right px-4 py-2 font-medium">{t("settings.sections.admin.users.actions", "Actions")}</th>
            </tr>
          </thead>
          <tbody>
            {users.map((u) => (
              <tr key={u.id} className="border-t hover:bg-muted/30">
                <td className="px-4 py-2">{u.username}</td>
                <td className="px-4 py-2">
                  <span className={`text-xs px-2 py-0.5 rounded-full ${
                    u.role === "admin" ? "bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200" : "bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-300"
                  }`}>
                    {u.role}
                  </span>
                </td>
                <td className="px-4 py-2 text-muted-foreground">{new Date(u.created_at * 1000).toLocaleDateString()}</td>
                <td className="px-4 py-2 text-right">
                  <div className="flex justify-end gap-1">
                    <Button variant="ghost" size="icon-xs" onClick={() => setEditUser(u)}>
                      <Pencil className="h-3.5 w-3.5" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon-xs"
                      className="text-destructive hover:text-destructive"
                      disabled={currentUser?.username === u.username}
                      onClick={() => handleDelete(u.username, fetchUsers, t)}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                </td>
              </tr>
            ))}
            {users.length === 0 && (
              <tr>
                <td colSpan={4} className="px-4 py-8 text-center text-muted-foreground">
                  {t("settings.sections.admin.users.noUsers", "No users found")}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <AddUserDialog open={addOpen} onOpenChange={setAddOpen} onDone={fetchUsers} />
      {editUser && <EditUserDialog user={editUser} onClose={() => setEditUser(null)} onDone={fetchUsers} />}
    </div>
  )
}

async function handleDelete(username: string, onDone: () => void, t: (key: string, opts?: Record<string, string>) => string) {
  if (!confirm(t("settings.sections.admin.users.deleteConfirm", { username }) || `Delete user "${username}"?`)) return
  try {
    await adminApi.deleteUser(username)
    onDone()
  } catch (err) {
    alert(String(err))
  }
}

// ── Add User Dialog ───────────────────────────────────────────────

function AddUserDialog({ open, onOpenChange, onDone }: { open: boolean; onOpenChange: (v: boolean) => void; onDone: () => void }) {
  const { t } = useTranslation()
  const [username, setUsername] = useState("")
  const [password, setPassword] = useState("")
  const [role, setRole] = useState("user")
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState("")

  useEffect(() => {
    if (open) { setUsername(""); setPassword(""); setRole("user"); setError("") }
  }, [open])

  async function submit() {
    if (!username || !password) { setError("Username and password are required"); return }
    setLoading(true)
    setError("")
    try {
      await adminApi.createUser(username, password, role)
      onDone()
      onOpenChange(false)
    } catch (err) {
      setError(String(err))
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("settings.sections.admin.users.addUser", "Add User")}</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-2">
            <Label>{t("settings.sections.admin.users.username", "Username")}</Label>
            <Input value={username} onChange={(e) => setUsername(e.target.value)} />
          </div>
          <div className="space-y-2">
            <Label>{t("settings.sections.admin.users.password", "Password")}</Label>
            <Input type="password" value={password} onChange={(e) => setPassword(e.target.value)} />
          </div>
          <div className="space-y-2">
            <Label>{t("settings.sections.admin.users.role", "Role")}</Label>
            <select
              className="flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm"
              value={role}
              onChange={(e) => setRole(e.target.value)}
            >
              <option value="user">user</option>
              <option value="admin">admin</option>
            </select>
          </div>
          {error && <p className="text-sm text-destructive">{error}</p>}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>{t("project.cancel")}</Button>
          <Button onClick={submit} disabled={loading}>{loading ? "..." : t("project.create")}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// ── Edit User Dialog ──────────────────────────────────────────────

function EditUserDialog({ user, onClose, onDone }: { user: adminApi.User; onClose: () => void; onDone: () => void }) {
  const { t } = useTranslation()
  const [password, setPassword] = useState("")
  const [role, setRole] = useState(user.role)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState("")

  async function submit() {
    setLoading(true)
    setError("")
    try {
      const patch: { password?: string; role?: string } = {}
      if (password) patch.password = password
      if (role !== user.role) patch.role = role
      await adminApi.updateUser(user.username, patch)
      onDone()
      onClose()
    } catch (err) {
      setError(String(err))
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open onOpenChange={onClose}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("settings.sections.admin.users.editUser", "Edit User")} — {user.username}</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-2">
            <Label>{t("settings.sections.admin.users.newPassword", "New Password")}</Label>
            <Input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder={t("settings.sections.admin.users.newPasswordPlaceholder", "Leave empty to keep")} />
          </div>
          <div className="space-y-2">
            <Label>{t("settings.sections.admin.users.role", "Role")}</Label>
            <select
              className="flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm"
              value={role}
              onChange={(e) => setRole(e.target.value)}
            >
              <option value="user">user</option>
              <option value="admin">admin</option>
            </select>
          </div>
          {error && <p className="text-sm text-destructive">{error}</p>}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>{t("project.cancel")}</Button>
          <Button onClick={submit} disabled={loading}>{loading ? "..." : t("settings.save")}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// ── Projects Tab ──────────────────────────────────────────────────

function ProjectsTab() {
  const { t } = useTranslation()
  const [projects, setProjects] = useState<ProjectInfo[]>([])
  const [users, setUsers] = useState<adminApi.User[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState("")
  const [reassigning, setReassigning] = useState<string | null>(null)

  const fetchData = useCallback(async () => {
    try {
      setError("")
      const [projList, userList] = await Promise.all([listProjects(), adminApi.listUsers()])
      setProjects(projList)
      setUsers(userList)
    } catch (err) {
      setError(String(err))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { fetchData() }, [fetchData])

  async function handleReassign(projectId: string, newUserId: string) {
    setReassigning(projectId)
    try {
      await adminApi.assignProject(projectId, newUserId)
      await fetchData()
    } catch (err) {
      alert(String(err))
    } finally {
      setReassigning(null)
    }
  }

  if (loading) {
    return <div className="flex items-center justify-center py-8"><Loader2 className="h-6 w-6 animate-spin" /></div>
  }

  return (
    <div className="space-y-4">
      {error && <p className="text-sm text-destructive">{error}</p>}

      <div className="rounded-lg border overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="text-left px-4 py-2 font-medium">{t("settings.sections.admin.projects.name", "Name")}</th>
              <th className="text-left px-4 py-2 font-medium">{t("settings.sections.admin.projects.path", "Path")}</th>
              <th className="text-left px-4 py-2 font-medium">{t("settings.sections.admin.projects.owner", "Owner")}</th>
              <th className="text-center px-4 py-2 font-medium">{t("settings.sections.admin.projects.hasWiki", "Wiki")}</th>
              <th className="text-right px-4 py-2 font-medium">{t("settings.sections.admin.projects.reassign", "Reassign")}</th>
            </tr>
          </thead>
          <tbody>
            {projects.map((p) => (
              <tr key={p.path} className="border-t hover:bg-muted/30">
                <td className="px-4 py-2 font-medium">{p.name}</td>
                <td className="px-4 py-2 text-muted-foreground max-w-48 truncate" title={p.path}>{p.path}</td>
                <td className="px-4 py-2">{p.owner || "-"}</td>
                <td className="px-4 py-2 text-center">{p.has_wiki ? "✓" : "-"}</td>
                <td className="px-4 py-2 text-right">
                  {p.id && !reassigning ? (
                    <select
                      className="h-8 rounded-md border bg-background px-2 text-xs"
                      value={users.find((u) => u.username === p.owner)?.id ?? ""}
                      onChange={(e) => {
                        if (e.target.value) handleReassign(p.id!, e.target.value)
                      }}
                    >
                      <option value="">-</option>
                      {users.map((u) => (
                        <option key={u.id} value={u.id}>{u.username}</option>
                      ))}
                    </select>
                  ) : reassigning === p.id ? (
                    <Loader2 className="h-4 w-4 animate-spin inline" />
                  ) : (
                    <span className="text-xs text-muted-foreground">-</span>
                  )}
                </td>
              </tr>
            ))}
            {projects.length === 0 && (
              <tr>
                <td colSpan={5} className="px-4 py-8 text-center text-muted-foreground">
                  {t("settings.sections.admin.projects.noProjects", "No projects found")}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
