// space module - manages workspace/space entities
export interface SpaceConfig {
  id: string;
  name: string;
  universeId: string;
  ownerId: string;
}

export class Space {
  constructor(private readonly config: SpaceConfig) {}
  
  get id(): string {
    return this.config.id;
  }
  
  get name(): string {
    return this.config.name;
  }
  
  async checkAccess(userId: string): Promise<boolean> {
    // placeholder access check
    return userId === this.config.ownerId;
  }
} 