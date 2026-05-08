import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { setAuthToken, apiPost } from "@/api/client"
import type { AuthUser } from "@/stores/auth-store"

interface LoginPageProps {
  onLogin: (user: AuthUser) => void
}

interface LoginResponse {
  token: string;
  user: AuthUser;
}

export function LoginPage({ onLogin }: LoginPageProps) {
  const { t } = useTranslation()
  const [username, setUsername] = useState("")
  const [password, setPassword] = useState("")
  const [error, setError] = useState("")
  const [loading, setLoading] = useState(false)

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setError("")
    setLoading(true)

    try {
      const data = await apiPost<LoginResponse>("/auth/login", { username, password })
      setAuthToken(data.token)
      onLogin(data.user)
    } catch (err) {
      const msg = err instanceof Error && err.message.startsWith("401")
        ? t("auth.invalidCredentials", "Invalid username or password")
        : t("auth.networkError", "Network error, please try again")
      setError(msg)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="flex h-screen items-center justify-center bg-background">
      <form
        onSubmit={handleSubmit}
        className="w-full max-w-sm space-y-4 rounded-lg border p-6 shadow-sm"
      >
        <h1 className="text-xl font-semibold text-center">
          LLM Wiki
        </h1>
        <div className="space-y-2">
          <Label htmlFor="username">{t("auth.username", "Username")}</Label>
          <Input
            id="username"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            placeholder={t("auth.usernamePlaceholder", "Enter username")}
            autoFocus
            required
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="password">{t("auth.password", "Password")}</Label>
          <Input
            id="password"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder={t("auth.passwordPlaceholder", "Enter password")}
            required
          />
        </div>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <Button type="submit" className="w-full" disabled={loading}>
          {loading ? t("auth.loggingIn", "Logging in...") : t("auth.login", "Login")}
        </Button>
      </form>
    </div>
  )
}
