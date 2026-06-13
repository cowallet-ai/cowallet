"use client";

import { useState, useCallback } from "react";

export type PhoneView = "home" | "wallet" | "agents" | "settings" | "keys" | "chat";
export type OnboardingStage = "hero" | "start" | "creating" | "name" | "ready";

export interface ChatMessage {
  id: string;
  type: "user" | "ai";
  text: string;
  intent?: IntentResult | null;
}

export interface IntentResult {
  kind: string;
  title: { zh: string; en: string };
  sub: { zh: string; en: string };
  yes: { zh: string; en: string };
  no: { zh: string; en: string };
}

export interface PhoneState {
  currentView: PhoneView;
  onboardingStage: OnboardingStage;
  showOnboarding: boolean;
  userName: string;
  chatMessages: ChatMessage[];
}

export function usePhoneState() {
  const [state, setState] = useState<PhoneState>({
    currentView: "home",
    onboardingStage: "hero",
    showOnboarding: true,
    userName: "",
    chatMessages: [],
  });

  const setView = useCallback((view: PhoneView) => {
    setState((s) => ({ ...s, currentView: view }));
  }, []);

  const setOnboardingStage = useCallback((stage: OnboardingStage) => {
    setState((s) => ({ ...s, onboardingStage: stage }));
  }, []);

  const dismissOnboarding = useCallback(() => {
    setState((s) => ({ ...s, showOnboarding: false, currentView: "home" }));
  }, []);

  const resetOnboarding = useCallback(() => {
    setState((s) => ({
      ...s,
      showOnboarding: true,
      onboardingStage: "hero",
      currentView: "home",
      chatMessages: [],
      userName: "",
    }));
  }, []);

  const setUserName = useCallback((name: string) => {
    setState((s) => ({ ...s, userName: name }));
  }, []);

  const addMessage = useCallback((msg: ChatMessage) => {
    setState((s) => ({ ...s, chatMessages: [...s.chatMessages, msg] }));
  }, []);

  const clearMessages = useCallback(() => {
    setState((s) => ({ ...s, chatMessages: [] }));
  }, []);

  return {
    state,
    setView,
    setOnboardingStage,
    dismissOnboarding,
    resetOnboarding,
    setUserName,
    addMessage,
    clearMessages,
  };
}
