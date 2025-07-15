import * as grpc from '@grpc/grpc-js';
import { render } from 'ink';
import pino from 'pino';
import React from 'react';
import { ServerStatus } from './components/ServerStatus.js';

interface ServeOptions {
  port: number;
  host: string;
  verbose: boolean;
}

const logger = pino({
  transport: {
    target: 'pino-pretty',
    options: {
      colorize: true,
    },
  },
});

export async function serve(options: ServeOptions): Promise<void> {
  logger.info('Starting destack-ts-system server...');
  
  // set up telemetry
  // setupTelemetry();
  
  // create gRPC server
  const server = new grpc.Server();
  
  // add a simple health check service for now
  server.addService(
    {
      check: {
        path: '/grpc.health.v1.Health/Check',
        requestStream: false,
        responseStream: false,
        requestSerialize: (value: any) => Buffer.from(JSON.stringify(value)),
        requestDeserialize: (value: Buffer) => JSON.parse(value.toString()),
        responseSerialize: (value: any) => Buffer.from(JSON.stringify(value)),
        responseDeserialize: (value: Buffer) => JSON.parse(value.toString()),
      },
    } as any,
    {
      check: (call: any, callback: any) => {
        callback(null, { status: 'SERVING' });
      },
    }
  );
  
  // bind server to address
  const address = `${options.host}:${options.port}`;
  
  server.bindAsync(address, grpc.ServerCredentials.createInsecure(), (error, port) => {
    if (error) {
      logger.error({ error }, 'Failed to bind server');
      process.exit(1);
    }
    
    logger.info({ address: `${options.host}:${port}` }, 'Server started');
    
    // render the Ink UI
    const app = render(
      <ServerStatus 
        host={options.host} 
        port={port} 
        status="running"
        verbose={options.verbose}
      />
    );
    
    // handle graceful shutdown
    const shutdown = () => {
      logger.info('Shutting down server...');
      app.unmount();
      server.tryShutdown(() => {
        logger.info('Server shut down successfully');
        process.exit(0);
      });
    };
    
    process.on('SIGINT', shutdown);
    process.on('SIGTERM', shutdown);
  });
} 