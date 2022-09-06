export type Tag = {
  type?: string;
  name: string;
  description?: string;
  metadata: Record<string, any>;
  created_at: string;
  updated_at: string;
  references_count?: number;
};

export type Taggable = {
  tags?: string[];
};
