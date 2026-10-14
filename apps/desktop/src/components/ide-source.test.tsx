import { describe, expect, it, vi } from 'vitest';
import { api } from '../api';
import { canWriteTarget, canRouteTarget, gatewayCapabilities, listAgentBackups } from './ide-source/ide-adapter';
vi.mock('../lib/tauri', () => ({ invoke: vi.fn() }));

describe('source workbench thin host adapter', () => {
  it('keeps backend now_ms backup timestamps in milliseconds', async () => {
    vi.spyOn(api, 'listAgentBackups').mockResolvedValueOnce([{ id: '1775000000000', created_at: 1775000000000, path: 'owned/backup' }]);
    const result = await listAgentBackups('opencode');
    expect(result.backups[0].createdAt).toBe(1775000000000);
    expect(new Date(result.backups[0].createdAt).getFullYear()).toBe(2026);
  });
  it.each(['claude-desktop', 'codex', 'grok-build'])('fails closed for %s even if backend supported flag regresses', id => {
    for (const protocol of ['OpenAI Chat', 'Anthropic Messages', 'Anthropic Messages (3P)', 'OpenAI Responses'])
      expect(canWriteTarget({id, installed: true, supported: true, protocol, configured: false, label: id, config_path: 'owned', configPath: 'owned', version: null, note: ''})).toBe(false);
  });
  it('allows only installed/supported Messages for Claude Code, not merely a supported flag', () => {
    const target = {id: 'claude-code', installed: true, supported: true, protocol: 'Anthropic Messages', configured: false, label: 'Claude Code', config_path: 'owned', configPath: 'owned', version: null, note: ''};
    expect(canWriteTarget(target)).toBe(true);
    expect(canWriteTarget({...target, installed: false})).toBe(false);
    expect(canWriteTarget({...target, supported: false})).toBe(false);
    expect(canWriteTarget({...target, protocol: 'OpenAI Chat'})).toBe(false);
    expect(canWriteTarget({...target, protocol: 'OpenAI Responses'})).toBe(false);
  });
  it('reports compiled capabilities while OFF without guessing missing DTO or widening writer contracts', () => {
    const caps = {
      ingress: [
        { protocol: 'openai_chat', endpoints: ['/v1/chat/completions'] },
        { protocol: 'anthropic_messages', endpoints: ['/v1/messages'] },
        { protocol: 'openai_responses', endpoints: ['/v1/responses', '/responses'] },
      ],
      backend_protocols: ['openai_chat', 'openai_responses', 'anthropic_messages'],
      ir_source: 'TiyGate / Apache-2.0',
    };
    const off = {is_running: false, port: 28319, master_key_preview: '', protocol_capabilities: caps};
    expect(gatewayCapabilities(off)).toEqual(caps);
    expect(gatewayCapabilities(null)).toBeNull();
    expect(gatewayCapabilities({...off, protocol_capabilities: undefined})).toBeNull();
    expect(gatewayCapabilities({...off, protocol_capabilities: null})).toBeNull();
    const target = {id: 'claude-code', installed: true, supported: true, protocol: 'Anthropic Messages', configured: false, label: 'Code', config_path: 'owned', configPath: 'owned', version: null, note: ''};
    expect(canRouteTarget(target, off)).toBe(true);
    expect(canRouteTarget(target, {...off, protocol_capabilities: undefined})).toBe(false);
    expect(canRouteTarget(target, {...off, protocol_capabilities: {...caps, ingress: []}})).toBe(false);
    expect(canRouteTarget({...target, protocol: 'Anthropic Messages unknown'}, off)).toBe(false);
    for (const id of ['claude-desktop', 'codex', 'grok-build']) {
      const blocked = {...target, id, protocol: id === 'claude-desktop' ? 'Anthropic Messages (3P)' : 'OpenAI Responses'};
      expect(canRouteTarget(blocked, off)).toBe(true);
      expect(canWriteTarget(blocked)).toBe(false);
    }
    expect(gatewayCapabilities({...off, protocol_capabilities: {...caps, ir_source: ''}})).toBeNull();
    expect(gatewayCapabilities({...off, protocol_capabilities: {...caps, ingress: [{protocol: 'openai_chat', endpoints: [123]}]} } as unknown as import('../types').GatewayStatusInfo)).toBeNull();
  });
});
