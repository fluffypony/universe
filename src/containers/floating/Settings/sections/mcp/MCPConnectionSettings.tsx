import { Typography } from '@app/components/elements/Typography';
import { Input } from '@app/components/elements/inputs/Input';
import {
    SettingsGroupWrapper,
    SettingsGroupTitle,
    SettingsGroupContent,
    SettingsGroup,
    SettingsGroupAction,
} from '../../components/SettingsGroup.styles';
import { useTranslation } from 'react-i18next';
import { useConfigCoreStore, setMcpPort, setMcpAllowedHostAddresses } from '@app/store';
import { useState, useEffect } from 'react';
import { Button } from '@app/components/elements/buttons/Button';
import styled from 'styled-components';

const HostListContainer = styled.div`
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 8px;
`;

const HostEntry = styled.div`
    display: flex;
    gap: 8px;
    align-items: center;
`;

const MCPConnectionSettings = () => {
    const { t } = useTranslation(['settings'], { useSuspense: false });
    const mcpEnabled = useConfigCoreStore((s) => s.mcp_enabled);
    const mcpPort = useConfigCoreStore((s) => s.mcp_port);
    const allowedHosts = useConfigCoreStore((s) => s.mcp_allowed_host_addresses);
    
    const [localPort, setLocalPort] = useState(mcpPort?.toString() || '0');
    const [newHost, setNewHost] = useState('');
    const [hostList, setHostList] = useState<string[]>(allowedHosts || ['127.0.0.1', '::1']);

    useEffect(() => {
        setLocalPort(mcpPort?.toString() || '0');
    }, [mcpPort]);

    useEffect(() => {
        setHostList(allowedHosts || ['127.0.0.1', '::1']);
    }, [allowedHosts]);

    const handlePortChange = (value: string) => {
        setLocalPort(value);
        const port = parseInt(value, 10);
        if (!isNaN(port) && port >= 0 && port <= 65535) {
            setMcpPort(port);
        }
    };

    const handleAddHost = () => {
        if (newHost.trim() && !hostList.includes(newHost.trim())) {
            const updatedHosts = [...hostList, newHost.trim()];
            setHostList(updatedHosts);
            setMcpAllowedHostAddresses(updatedHosts);
            setNewHost('');
        }
    };

    const handleRemoveHost = (host: string) => {
        const updatedHosts = hostList.filter(h => h !== host);
        setHostList(updatedHosts);
        setMcpAllowedHostAddresses(updatedHosts);
    };

    if (!mcpEnabled) return null;

    return (
        <SettingsGroupWrapper $advanced>
            <SettingsGroup>
                <SettingsGroupContent>
                    <SettingsGroupTitle>
                        <Typography variant="h6">{t('mcp.connection.port.title', 'Server Port')}</Typography>
                    </SettingsGroupTitle>
                    <Typography variant="p">
                        {t('mcp.connection.port.description', 'Port for the MCP server (0 = random available port)')}
                    </Typography>
                </SettingsGroupContent>
                <SettingsGroupAction>
                    <Input
                        type="number"
                        value={localPort}
                        onChange={(e) => handlePortChange(e.target.value)}
                        placeholder="0"
                        min="0"
                        max="65535"
                        style={{ width: '100px' }}
                    />
                </SettingsGroupAction>
            </SettingsGroup>

            <SettingsGroup>
                <SettingsGroupContent>
                    <SettingsGroupTitle>
                        <Typography variant="h6">{t('mcp.connection.allowed-hosts.title', 'Allowed Host Addresses')}</Typography>
                    </SettingsGroupTitle>
                    <Typography variant="p">
                        {t('mcp.connection.allowed-hosts.description', 'Host addresses that are allowed to connect to the MCP server')}
                    </Typography>
                    
                    <HostListContainer>
                        {hostList.map((host, index) => (
                            <HostEntry key={index}>
                                <Typography variant="p" style={{ flex: 1 }}>{host}</Typography>
                                <Button
                                    onClick={() => handleRemoveHost(host)}
                                    color="error"
                                    size="small"
                                    disabled={hostList.length <= 1}
                                >
                                    {t('mcp.connection.remove', 'Remove')}
                                </Button>
                            </HostEntry>
                        ))}
                        
                        <HostEntry>
                            <Input
                                value={newHost}
                                onChange={(e) => setNewHost(e.target.value)}
                                placeholder={t('mcp.connection.new-host-placeholder', 'Enter host address (e.g., 192.168.1.100)')}
                                style={{ flex: 1 }}
                                onKeyPress={(e) => {
                                    if (e.key === 'Enter') {
                                        handleAddHost();
                                    }
                                }}
                            />
                            <Button
                                onClick={handleAddHost}
                                variant="secondary"
                                size="small"
                                disabled={!newHost.trim() || hostList.includes(newHost.trim())}
                            >
                                {t('mcp.connection.add', 'Add')}
                            </Button>
                        </HostEntry>
                    </HostListContainer>
                </SettingsGroupContent>
            </SettingsGroup>
        </SettingsGroupWrapper>
    );
};

export default MCPConnectionSettings;
