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
import { useConfigCoreStore, setMcpAllowWalletSend, setMcpAuditLogging } from '@app/store';

const MCPSecuritySettings = () => {
    const { t } = useTranslation(['settings'], { useSuspense: false });
    const mcpEnabled = useConfigCoreStore((s) => s.mcp_enabled);
    const allowWalletSend = useConfigCoreStore((s) => s.mcp_allow_wallet_send);
    const auditLogging = useConfigCoreStore((s) => s.mcp_audit_logging);

    if (!mcpEnabled) return null;

    return (
        <SettingsGroupWrapper>
            <SettingsGroup>
                <SettingsGroupContent>
                    <SettingsGroupTitle>
                        <Typography variant="h6">{t('mcp.security.wallet-send.title', 'Allow Wallet Transactions')}</Typography>
                    </SettingsGroupTitle>
                    <Typography variant="p">
                        {t('mcp.security.wallet-send.description', 'Allow AI agents to send Tari transactions from your wallet. HIGH SECURITY RISK - Only enable for trusted agents.')}
                    </Typography>
                </SettingsGroupContent>
                <SettingsGroupAction>
                    <ToggleSwitch 
                        checked={allowWalletSend} 
                        onChange={({ target }) => setMcpAllowWalletSend(target.checked)} 
                    />
                </SettingsGroupAction>
            </SettingsGroup>

            <SettingsGroup>
                <SettingsGroupContent>
                    <SettingsGroupTitle>
                        <Typography variant="h6">{t('mcp.security.audit-logging.title', 'Enable Audit Logging')}</Typography>
                    </SettingsGroupTitle>
                    <Typography variant="p">
                        {t('mcp.security.audit-logging.description', 'Log all MCP operations for security monitoring and debugging')}
                    </Typography>
                </SettingsGroupContent>
                <SettingsGroupAction>
                    <ToggleSwitch 
                        checked={auditLogging} 
                        onChange={({ target }) => setMcpAuditLogging(target.checked)} 
                    />
                </SettingsGroupAction>
            </SettingsGroup>
        </SettingsGroupWrapper>
    );
};

export default MCPSecuritySettings;
