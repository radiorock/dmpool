import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useAuthStore = defineStore('auth', () => {
  const isAuthenticated = ref<boolean>(false);
  const token = ref<string>('');

  function login(authToken: string) {
    token.value = authToken;
    isAuthenticated.value = true;
    localStorage.setItem('admin_token', authToken);
  }

  function logout() {
    token.value = '';
    isAuthenticated.value = false;
    localStorage.removeItem('admin_token');
  }

  function checkAuth() {
    const storedToken = localStorage.getItem('admin_token');
    if (storedToken) {
      token.value = storedToken;
      isAuthenticated.value = true;
    }
  }

  return {
    isAuthenticated,
    token,
    login,
    logout,
    checkAuth,
  };
});
