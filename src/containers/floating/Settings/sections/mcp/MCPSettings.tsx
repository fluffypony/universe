import { SettingsGroupWrapper } from '@app/containers/floating/Settings/components/SettingsGroup.styles';
import MCPServerSettings from './MCPServerSettings';
import MCPSecuritySettings from './MCPSecuritySettings';
import MCPConnectionSettings from './MCPConnectionSettings';

export const MCPSettings = () => {
    return (
        <>
            <MCPServerSettings />
            <MCPSecuritySettings />
            <MCPConnectionSettings />
        </>
    );
};
