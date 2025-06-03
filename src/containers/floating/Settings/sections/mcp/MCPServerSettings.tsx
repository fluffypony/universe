import { ToggleSwitch } from '@app/components/elements/ToggleSwitch';
import { Typography } from '@app/components/elements/Typography';
import {
    SettingsGroupWrapper,
    SettingsGroupTitle,
    SettingsGroupContent,
    SettingsGroup,
    SettingsGroupAction,
} from '../../components/SettingsGroup.styles';
import { useTranslation } from 'react-i18next';
import { useConfigCoreStore, setMcpEnabled } from '@app/store';
import { Button } from '@app/components/elements/buttons/Button';
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { addToast } from '@app/components/ToastStack/useToastStore';

const MCPServerSettings = () => {
    const { t } = useTranslation(['settings'], { useSuspense: false });
    const mcpEnabled = useConfigCoreStore((s) => s.mcp_enabled);
    const [isRestarting, setIsRestarting] = useState(false);

    const handleToggle = async (enabled: boolean) => {
        setMcpEnabled(enabled);
        
        if (enabled) {
            addToast({
                title: t('mcp.server.title', 'MCP Server'),
                text: t('mcp.server.enabled-message', 'MCP server will start with next application restart'),
                type: 'info'
            });
        } else {
            addToast({
                title: t('mcp.server.title', 'MCP Server'),
                text: t('mcp.server.disabled-message', 'MCP server disabled'),
                type: 'info'
            });
        }
    };

    const handleRestartServer = async () => {
        setIsRestarting(true);
        try {
            await invoke('restart_mcp_server');
            addToast({
                title: t('mcp.server.restart.title', 'Restart MCP Server'),
                text: t('mcp.server.restart-success', 'MCP server restarted successfully'),
                type: 'success'
            });
        } catch (error) {
            addToast({
                title: t('mcp.server.restart.title', 'Restart MCP Server'),
                text: t('mcp.server.restart-error', 'Failed to restart MCP server: ') + error,
                type: 'error'
            });
        } finally {
            setIsRestarting(false);
        }
    };

    return (
        <SettingsGroupWrapper>
            <SettingsGroup>
                <SettingsGroupContent>
                    <SettingsGroupTitle>
                        <Typography variant="h6">{t('mcp.server.title', 'Enable MCP Server')}</Typography>
                    </SettingsGroupTitle>
                    <Typography variant="p">
                        {t('mcp.server.description', 'Allow AI agents to interact with Tari Universe through the Model Context Protocol')}
                    </Typography>
                </SettingsGroupContent>
                <SettingsGroupAction>
                    <ToggleSwitch checked={mcpEnabled} onChange={({ target }) => handleToggle(target.checked)} />
                </SettingsGroupAction>
            </SettingsGroup>
            
            {mcpEnabled && (
                <SettingsGroup>
                    <SettingsGroupContent>
                        <SettingsGroupTitle>
                            <Typography variant="h6">{t('mcp.server.restart.title', 'Restart MCP Server')}</Typography>
                        </SettingsGroupTitle>
                        <Typography variant="p">
                            {t('mcp.server.restart.description', 'Restart the MCP server with current configuration')}
                        </Typography>
                    </SettingsGroupContent>
                    <SettingsGroupAction>
                        <Button 
                            onClick={handleRestartServer} 
                            disabled={isRestarting}
                            variant="secondary"
                            size="small"
                        >
                            {isRestarting ? t('mcp.server.restarting', 'Restarting...') : t('mcp.server.restart-button', 'Restart Server')}
                        </Button>
                    </SettingsGroupAction>
                </SettingsGroup>
            )}
        </SettingsGroupWrapper>
    );
};

export default MCPServerSettings;
