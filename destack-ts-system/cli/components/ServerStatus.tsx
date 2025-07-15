import React from 'react';
import { Box, Text } from 'ink';

interface ServerStatusProps {
  host: string;
  port: number;
  status: 'starting' | 'running' | 'stopping' | 'error';
  verbose: boolean;
}

export function ServerStatus({ host, port, status, verbose }: ServerStatusProps) {
  const statusColor = {
    starting: 'yellow',
    running: 'green',
    stopping: 'yellow',
    error: 'red',
  }[status];

  return (
    <Box flexDirection="column" padding={1}>
      <Box>
        <Text bold>📡 destack-ts-system Server</Text>
      </Box>
      
      <Box marginTop={1}>
        <Text>Status: </Text>
        <Text color={statusColor} bold>
          {status.toUpperCase()}
        </Text>
      </Box>
      
      <Box>
        <Text>Address: </Text>
        <Text color="cyan">
          {host}:{port}
        </Text>
      </Box>
      
      {verbose && (
        <Box marginTop={1} flexDirection="column">
          <Text dimColor>Press Ctrl+C to stop the server</Text>
          <Text dimColor>Verbose mode enabled</Text>
        </Box>
      )}
    </Box>
  );
} 