import { create } from 'zustand';

export interface AuthUser {
  id: string;
  username: string;
  role: string;
  created_at: number;
}

interface AuthState {
  user: AuthUser | null;
  isAdmin: boolean;
  setUser: (user: AuthUser | null) => void;
  clear: () => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  isAdmin: false,
  setUser: (user) => set((state) => {
    if (state.user?.id === user?.id && state.user?.role === user?.role) return state;
    return { user, isAdmin: user?.role === 'admin' };
  }),
  clear: () => set({ user: null, isAdmin: false }),
}));
