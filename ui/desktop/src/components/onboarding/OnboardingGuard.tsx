import React, { useEffect } from 'react';
import { useModelAndProvider } from '../ModelAndProviderContext';
import { acpListProviderDetails, acpReadDefaults, acpSaveDefaults } from '../../acp/providers';

interface OnboardingGuardProps {
  children: React.ReactNode;
}

export default function OnboardingGuard({ children }: OnboardingGuardProps) {
  const { refreshCurrentModelAndProvider } = useModelAndProvider();

  useEffect(() => {
    (async () => {
      try {
        const { providerId: provider } = await acpReadDefaults();
        if (provider?.trim()) return;

        // Auto-configure local default provider in the background if none is configured
        const providers = await acpListProviderDetails().catch(() => []);
        const local =
          providers.find((p) => p.name === 'ollama') ||
          providers.find((p) => p.name === 'local') ||
          providers.find((p) => p.name === 'openai') ||
          providers[0];

        if (local) {
          const resolvedModel = local.metadata?.default_model ?? null;
          await acpSaveDefaults(local.name, resolvedModel).catch(() => {});
          await refreshCurrentModelAndProvider().catch(() => {});
        }
      } catch (error) {
        console.error('Failed to auto-configure local provider:', error);
      }
    })();
  }, [refreshCurrentModelAndProvider]);

  // Bypass the provider selector screen entirely and view the direct app
  return <>{children}</>;
}
