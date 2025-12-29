import { Link, useLocation } from 'react-router-dom';
import { cn } from '@/lib/utils';

export function Navbar() {
  const location = useLocation();

  const navLinks = [
    { to: '/', label: 'Dashboard' },
    { to: '/agents', label: 'Agents' },
  ];

  return (
    <nav className="border-b bg-card">
      <div className="container mx-auto max-w-7xl flex items-center justify-between px-4 py-4">
        <Link to="/" className="text-xl font-bold text-primary">
          Orchestrate
        </Link>
        <div className="flex gap-6">
          {navLinks.map((link) => (
            <Link
              key={link.to}
              to={link.to}
              className={cn(
                'text-sm transition-colors hover:text-foreground',
                location.pathname === link.to
                  ? 'text-foreground'
                  : 'text-muted-foreground'
              )}
            >
              {link.label}
            </Link>
          ))}
        </div>
      </div>
    </nav>
  );
}
